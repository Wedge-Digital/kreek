//! Réserver, rendre, envoyer, confirmer — ce que les deux déclencheurs
//! partagent.
//!
//! # Un envoi par destinataire, jamais groupé
//!
//! `IEmailService::send` prend un `Vec<String>`, ce qui invite à expédier trente
//! coachs en un appel. **Ce serait mettre les trente adresses en clair dans
//! l'en-tête que chacun reçoit** — un annuaire de l'espace distribué à tout le
//! monde. R4 impose de toute façon un corps personnalisé : chacun voit ses
//! matchs, ou la ligne qui dit qu'il n'en a pas.
//!
//! # Aucune transaction n'enveloppe les quatre étapes
//!
//! C'est délibéré. Un appel réseau se produit entre la réservation et la
//! confirmation, et tenir une transaction ouverte pendant un aller-retour HTTP
//! est précisément ce que les garde-fous de la carte 317 cherchaient à
//! empêcher. La règle de transaction unique du CLAUDE.md vise les projections
//! event-sourcées, pas ceci.
//!
//! # Un échec unitaire n'interrompt pas la boucle
//!
//! Le destinataire est compté en `failed`, sa ligne reste à `sent_at NULL`, et
//! on continue. Une adresse invalide ne doit pas priver les vingt-neuf autres
//! de leur e-mail. La ligne restée à `NULL` est l'échec constaté que R1
//! demande — et R9 interdit de la rejouer le lendemain.

use crate::app::competitions::domain::notification_delivery::{DeliveryKey, NotificationType};
use crate::app::competitions::domain::notification_schedule::RoundRef;
use crate::app::competitions::io::email::notification_emails::{
    FixtureVm, ParticipationVm, RegistrationDeadlineEmail, RegistrationOpenEmail,
    RoundClosingEmail, RoundEveEmail,
};
use crate::app::competitions::io::repository::notification_delivery_repository::NotificationDeliveryRepository;
use crate::app::competitions::ports::{ICompetitionSpaceMemberPort, ITeamInfoPort};
use crate::app::competitions::use_cases::notification_recipients::{
    self, Recipient, RoundParticipation, SeasonContext,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::email::IEmailService;
use askama::Template;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DispatchOutcome {
    pub sent: usize,
    pub skipped_already_sent: usize,
    pub failed: usize,
}

pub struct DispatchDeps<'a> {
    pub teams: &'a dyn ITeamInfoPort,
    pub members: &'a dyn ICompetitionSpaceMemberPort,
    pub journal: &'a NotificationDeliveryRepository,
    pub email: &'a dyn IEmailService,
    pub app_url: &'a str,
}

/// Ce que le gabarit a besoin de savoir de la compétition, et que le
/// destinataire ne porte pas.
pub struct DispatchLabels {
    pub competition_name: String,
    pub season_name: String,
    pub space_name: String,
    pub admin_name: String,
    pub competition_url: String,
    pub registration_deadline: String,
    pub remaining_slots: String,
}

// L'attribut tient sur **une** ligne : l'axe 11 ne lit que celle qui précède
// immédiatement la signature, et un attribut réparti sur quatre lignes se
// termine par `)]`, qui ne contient pas « instrument ».
#[tracing::instrument(skip_all, fields(notif = ?notification, season = %season.season_id))]
pub async fn dispatch(
    notification: NotificationType,
    season: &SeasonContext<'_>,
    round: Option<&RoundRef>,
    target_date: &DateString,
    labels: &DispatchLabels,
    deps: &DispatchDeps<'_>,
) -> DispatchOutcome {
    let destinataires =
        notification_recipients::resolve(notification, season, round, deps.teams, deps.members)
            .await;

    let mut bilan = DispatchOutcome::default();
    for d in destinataires {
        traiter(
            notification,
            season,
            round,
            target_date,
            labels,
            deps,
            d,
            &mut bilan,
        )
        .await;
    }

    tracing::info!(
        sent = bilan.sent,
        skipped = bilan.skipped_already_sent,
        failed = bilan.failed,
        "notifications expédiées"
    );
    bilan
}

#[allow(clippy::too_many_arguments)]
async fn traiter(
    notification: NotificationType,
    season: &SeasonContext<'_>,
    round: Option<&RoundRef>,
    target_date: &DateString,
    labels: &DispatchLabels,
    deps: &DispatchDeps<'_>,
    d: Recipient,
    bilan: &mut DispatchOutcome,
) {
    let Some(cle) = cle(notification, season, round, target_date, &d) else {
        bilan.failed += 1;
        return;
    };

    // 1. Réserver. Zéro ligne insérée signifie « déjà envoyé » : la base
    //    tranche, pas le code, et c'est tout R3.
    match deps.journal.claim(&cle).await {
        Ok(true) => {}
        Ok(false) => {
            bilan.skipped_already_sent += 1;
            return;
        }
        Err(e) => {
            tracing::error!(coach = %d.coach_id, "réservation impossible : {e}");
            bilan.failed += 1;
            return;
        }
    }

    // 2 et 3. Rendre, puis envoyer — un destinataire à la fois.
    let (sujet, html) = rendre(notification, round, labels, deps.app_url, &d);
    // arch:ok envoi d'e-mail, pas une émission d'évènement — le bus n'est pas en cause
    match deps.email.send(vec![d.email.clone()], sujet, html).await {
        // 4. Confirmer. Une confirmation en échec laisse la ligne à `NULL` :
        //    l'e-mail est parti, mais on ne peut pas l'attester, et le prochain
        //    passage ne le rejouera pas pour autant — la ligne existe.
        Ok(()) => {
            if let Err(e) = deps.journal.confirm(&cle).await {
                tracing::error!(coach = %d.coach_id, "confirmation impossible : {e}");
            }
            bilan.sent += 1;
        }
        Err(e) => {
            tracing::warn!(coach = %d.coach_id, "envoi en échec : {e}");
            bilan.failed += 1;
        }
    }
}

fn cle(
    notification: NotificationType,
    season: &SeasonContext<'_>,
    round: Option<&RoundRef>,
    target_date: &DateString,
    d: &Recipient,
) -> Option<DeliveryKey> {
    Some(DeliveryKey {
        notification_type: notification,
        season_id: SeasonId::try_new(season.season_id).ok()?,
        round_id: round.map(|r| r.round_id.clone()),
        target_date: target_date.clone(),
        coach_id: CoachId::try_new(&d.coach_id).ok()?,
    })
}

fn participation_vm(p: &RoundParticipation) -> ParticipationVm {
    match p {
        RoundParticipation::NotPlaying => ParticipationVm::NotPlaying,
        RoundParticipation::Playing(f) => ParticipationVm::Playing(
            f.iter()
                .map(|x| FixtureVm {
                    team_name: x.team_name.clone(),
                    home_team: x.home_team.clone(),
                    away_team: x.away_team.clone(),
                })
                .collect(),
        ),
    }
}

fn rendre(
    notification: NotificationType,
    round: Option<&RoundRef>,
    l: &DispatchLabels,
    app_url: &str,
    d: &Recipient,
) -> (String, String) {
    match notification {
        NotificationType::RoundEve => veille(round, l, app_url, d),
        NotificationType::RoundClosing => cloture(round, l, app_url, d),
        NotificationType::RegistrationOpen => ouverture(l, app_url, d),
        NotificationType::RegistrationDeadline => date_limite(l, app_url, d),
    }
}

/// Un rendu en échec ne peut pas venir des données : les gabarits sont vérifiés
/// à la compilation. Le corps vide qui en sortirait serait pourtant envoyé — on
/// préfère un sujet seul à un `unwrap` qui ferait tomber tout le cron.
fn rendu(r: Result<String, askama::Error>) -> String {
    r.unwrap_or_else(|e| {
        tracing::error!("rendu de gabarit en échec : {e}");
        String::new()
    })
}

fn veille(
    round: Option<&RoundRef>,
    l: &DispatchLabels,
    app_url: &str,
    d: &Recipient,
) -> (String, String) {
    let r = round.expect("une veille de journée porte toujours sa journée");
    let html = rendu(
        RoundEveEmail {
            app_url: app_url.to_string(),
            coach_name: d.coach_name.clone(),
            competition_name: l.competition_name.clone(),
            competition_url: l.competition_url.clone(),
            round_name: r.round_name.to_string(),
            date_start: r.date_start.as_ref().to_string(),
            date_end: r.date_end.as_ref().map(|x| x.as_ref().to_string()),
            participation: participation_vm(&d.participation),
        }
        .render(),
    );
    (
        format!("{} démarre demain — {}", r.round_name, l.competition_name),
        html,
    )
}

fn cloture(
    round: Option<&RoundRef>,
    l: &DispatchLabels,
    app_url: &str,
    d: &Recipient,
) -> (String, String) {
    let r = round.expect("une clôture de journée porte toujours sa journée");
    let html = rendu(
        RoundClosingEmail {
            app_url: app_url.to_string(),
            coach_name: d.coach_name.clone(),
            competition_name: l.competition_name.clone(),
            competition_url: l.competition_url.clone(),
            round_name: r.round_name.to_string(),
            date_end: r
                .date_end
                .as_ref()
                .map(|x| x.as_ref().to_string())
                .unwrap_or_default(),
            participation: participation_vm(&d.participation),
        }
        .render(),
    );
    (
        format!("Plus que deux jours pour jouer {}", r.round_name),
        html,
    )
}

fn ouverture(l: &DispatchLabels, app_url: &str, d: &Recipient) -> (String, String) {
    let html = rendu(
        RegistrationOpenEmail {
            app_url: app_url.to_string(),
            coach_name: d.coach_name.clone(),
            admin_name: l.admin_name.clone(),
            space_name: l.space_name.clone(),
            competition_name: l.competition_name.clone(),
            season_name: l.season_name.clone(),
            competition_url: l.competition_url.clone(),
            registration_deadline: l.registration_deadline.clone(),
        }
        .render(),
    );
    (
        format!("Les inscriptions à {} sont ouvertes", l.competition_name),
        html,
    )
}

fn date_limite(l: &DispatchLabels, app_url: &str, d: &Recipient) -> (String, String) {
    let html = rendu(
        RegistrationDeadlineEmail {
            app_url: app_url.to_string(),
            coach_name: d.coach_name.clone(),
            admin_name: l.admin_name.clone(),
            competition_name: l.competition_name.clone(),
            season_name: l.season_name.clone(),
            competition_url: l.competition_url.clone(),
            registration_deadline: l.registration_deadline.clone(),
            remaining_slots: l.remaining_slots.clone(),
        }
        .render(),
    );
    (
        format!(
            "Plus que trois jours pour s'inscrire à {}",
            l.competition_name
        ),
        html,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::MatchDayName;
    use crate::app::competitions::domain::match_day::{MatchDayType, Pairing};
    use crate::app::competitions::ports::{SpaceMemberDto, TeamInfoDto};
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::common::services::email::EmailError;
    use async_trait::async_trait;
    use std::sync::Mutex;

    // ── L'espion ─────────────────────────────────────────────────────────────

    #[derive(Default)]
    struct Espion {
        envois: Mutex<Vec<(Vec<String>, String)>>,
        echoue: bool,
    }

    #[async_trait]
    impl IEmailService for Espion {
        async fn send(&self, to: Vec<String>, _: String, html: String) -> Result<(), EmailError> {
            self.envois.lock().unwrap().push((to, html));
            if self.echoue {
                return Err(EmailError::Network("panne simulée".into()));
            }
            Ok(())
        }
    }

    struct Membres(Vec<SpaceMemberDto>);
    #[async_trait]
    impl ICompetitionSpaceMemberPort for Membres {
        async fn list_space_members(&self, _: &SpaceId) -> Vec<SpaceMemberDto> {
            self.0.clone()
        }
        async fn find_member_profile(
            &self,
            _: &CoachId,
            _: &SpaceId,
        ) -> Option<crate::app::shared_kernel::identity::authorization::SpaceProfile> {
            None
        }
        async fn find_all_spaces(
            &self,
        ) -> Vec<crate::app::shared_kernel::identity::space_definition::SpaceDefinition> {
            Vec::new()
        }
    }

    struct Equipes(Vec<TeamInfoDto>);
    #[async_trait]
    impl ITeamInfoPort for Equipes {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.clone())
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(Vec::new())
        }
    }

    // ── Fixtures ─────────────────────────────────────────────────────────────

    const SAISON: &str = "01KZVCKDG19DXZHJA295WSJGMV";
    const ESPACE: &str = "01KZVCJZ35JZWQB0KNTA9JJEPX";
    const ALICE: &str = "01KZVCKDG19DXZHJA295WSJGM1";
    const BOB: &str = "01KZVCKDG19DXZHJA295WSJGM2";
    const T1: &str = "01KZVCKDG19DXZHJA295WSJGT1";
    const T2: &str = "01KZVCKDG19DXZHJA295WSJGT2";
    const T3: &str = "01KZVCKDG19DXZHJA295WSJGT3";

    fn membre(id: &str, nom: &str) -> SpaceMemberDto {
        SpaceMemberDto {
            coach_id: id.into(),
            coach_name: nom.into(),
            email: format!("{nom}@example.test"),
        }
    }

    fn equipe(team_id: &str, nom: &str, coach: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_id: team_id.into(),
            team_name: nom.into(),
            coach_id: coach.into(),
            coach_name: String::new(),
            roster_name: String::new(),
            logo_url: None,
        }
    }

    fn journee(pairings: Vec<Pairing>) -> RoundRef {
        RoundRef {
            round_id: MatchId::try_new("01KZVCKDG19DXZHJA295WSJGMX").unwrap(),
            round_name: MatchDayName::try_new("Journée 3").unwrap(),
            date_start: DateString::try_new("2026-09-11").unwrap(),
            date_end: None,
            day_type: MatchDayType::FixedDate,
            pairings,
        }
    }

    fn appariement(home: &str, away: &str) -> Pairing {
        Pairing {
            id: PairingId::try_new("01KZVCKDG19DXZHJA295WSJGP1").unwrap(),
            home_team_id: TeamId::try_new(home).unwrap(),
            away_team_id: TeamId::try_new(away).unwrap(),
        }
    }

    fn labels() -> DispatchLabels {
        DispatchLabels {
            competition_name: "Coupe de Fer".into(),
            season_name: "Saison 1".into(),
            space_name: "Ligue du Nord".into(),
            admin_name: "Nobbla".into(),
            competition_url: "https://kreek.example/c".into(),
            registration_deadline: "20/09/2026".into(),
            remaining_slots: "3".into(),
        }
    }

    async fn expedier(
        pool: &sqlx::PgPool,
        espion: &Espion,
        round: &RoundRef,
        date: &str,
        equipes: &Equipes,
        membres: &Membres,
    ) -> DispatchOutcome {
        let espace = SpaceId::try_new(ESPACE).unwrap();
        let journal = NotificationDeliveryRepository::new(pool.clone());
        let deps = DispatchDeps {
            teams: equipes,
            members: membres,
            journal: &journal,
            email: espion,
            app_url: "https://kreek.example",
        };
        let season = SeasonContext {
            space_id: &espace,
            season_id: SAISON,
            invitations: None,
        };
        dispatch(
            NotificationType::RoundEve,
            &season,
            Some(round),
            &DateString::try_new(date).unwrap(),
            &labels(),
            &deps,
        )
        .await
    }

    async fn lignes(pool: &sqlx::PgPool) -> Vec<(String, Option<time::OffsetDateTime>)> {
        sqlx::query_as("SELECT coach_id, sent_at FROM competition_notification_deliveries")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    // ── R3 ───────────────────────────────────────────────────────────────────

    /// Deux exécutions le même jour : la seconde ne renvoie rien. Ce n'est pas
    /// le code qui le décide — c'est l'index unique du journal.
    #[sqlx::test]
    async fn deux_executions_le_meme_jour_n_envoient_qu_un_email_par_coach(pool: sqlx::PgPool) {
        let espion = Espion::default();
        let membres = Membres(vec![membre(ALICE, "alice")]);
        let equipes = Equipes(vec![equipe(T1, "Les Uns", ALICE)]);
        let r = journee(vec![]);

        let un = expedier(&pool, &espion, &r, "2026-09-10", &equipes, &membres).await;
        let deux = expedier(&pool, &espion, &r, "2026-09-10", &equipes, &membres).await;

        assert_eq!(un.sent, 1);
        assert_eq!(deux.sent, 0);
        assert_eq!(deux.skipped_already_sent, 1);
        assert_eq!(espion.envois.lock().unwrap().len(), 1);
    }

    // ── R1 et R9 ─────────────────────────────────────────────────────────────

    /// L'envoi échoue : la ligne existe et reste à `NULL`. C'est l'échec
    /// **constaté** que R1 demande.
    #[sqlx::test]
    async fn un_envoi_en_echec_laisse_sa_ligne_sans_date(pool: sqlx::PgPool) {
        let espion = Espion {
            echoue: true,
            ..Default::default()
        };
        let membres = Membres(vec![membre(ALICE, "alice")]);
        let equipes = Equipes(vec![equipe(T1, "Les Uns", ALICE)]);

        let bilan = expedier(
            &pool,
            &espion,
            &journee(vec![]),
            "2026-09-10",
            &equipes,
            &membres,
        )
        .await;

        assert_eq!(bilan.failed, 1);
        assert_eq!(bilan.sent, 0);
        let l = lignes(&pool).await;
        assert_eq!(l.len(), 1, "la réservation subsiste");
        assert!(l[0].1.is_none(), "sent_at doit rester NULL");
    }

    /// **Le garde-fou de R9.** Le lendemain, la ligne restée à `NULL` n'est pas
    /// rejouée : `dispatch` ne sait même pas qu'elle existe, et la clé du jour
    /// est déjà prise. Si quelqu'un ajoutait un rattrapage, ce test rougirait.
    #[sqlx::test]
    async fn une_ligne_en_echec_n_est_pas_rejouee_le_lendemain(pool: sqlx::PgPool) {
        let membres = Membres(vec![membre(ALICE, "alice")]);
        let equipes = Equipes(vec![equipe(T1, "Les Uns", ALICE)]);
        let r = journee(vec![]);

        let echec = Espion {
            echoue: true,
            ..Default::default()
        };
        expedier(&pool, &echec, &r, "2026-09-10", &equipes, &membres).await;

        // Le lendemain, même journée visée : la clé est identique, donc prise.
        let lendemain = Espion::default();
        let bilan = expedier(&pool, &lendemain, &r, "2026-09-10", &equipes, &membres).await;

        assert_eq!(bilan.sent, 0, "aucun rattrapage — R9");
        assert_eq!(bilan.skipped_already_sent, 1);
        assert!(lendemain.envois.lock().unwrap().is_empty());
    }

    // ── R2 ───────────────────────────────────────────────────────────────────

    /// Une journée décalée change `target_date`, donc la clé, donc un second
    /// e-mail part. Aucune ligne de code n'est consacrée à ce réarmement : il
    /// vient de la forme de la clé.
    #[sqlx::test]
    async fn une_journee_decalee_reamorce_la_notification(pool: sqlx::PgPool) {
        let espion = Espion::default();
        let membres = Membres(vec![membre(ALICE, "alice")]);
        let equipes = Equipes(vec![equipe(T1, "Les Uns", ALICE)]);
        let r = journee(vec![]);

        expedier(&pool, &espion, &r, "2026-09-10", &equipes, &membres).await;
        let apres = expedier(&pool, &espion, &r, "2026-09-11", &equipes, &membres).await;

        assert_eq!(apres.sent, 1, "la date visée a changé — R2");
        assert_eq!(espion.envois.lock().unwrap().len(), 2);
    }

    // ── R4 ───────────────────────────────────────────────────────────────────

    /// Un coach à deux équipes reçoit **un** e-mail — la clé ne porte pas
    /// d'équipe — et il doit y voir ses **deux** matchs.
    #[sqlx::test]
    async fn un_coach_a_deux_equipes_recoit_un_email_listant_deux_matchs(pool: sqlx::PgPool) {
        let espion = Espion::default();
        let membres = Membres(vec![membre(ALICE, "alice"), membre(BOB, "bob")]);
        let equipes = Equipes(vec![
            equipe(T1, "Les Uns", ALICE),
            equipe(T2, "Les Deux", ALICE),
            equipe(T3, "Les Trois", BOB),
        ]);
        let r = journee(vec![appariement(T1, T3), appariement(T3, T2)]);

        let bilan = expedier(&pool, &espion, &r, "2026-09-10", &equipes, &membres).await;

        assert_eq!(bilan.sent, 2, "un e-mail par coach, pas par équipe");
        let envois = espion.envois.lock().unwrap();
        let (_, html) = envois
            .iter()
            .find(|(to, _)| to[0].starts_with("alice"))
            .expect("alice doit être servie");
        assert!(html.contains("Les Uns") && html.contains("Les Deux"));
        assert!(html.contains("Tes matchs"), "le pluriel doit apparaître");
    }

    /// Chaque envoi ne porte **qu'une** adresse. Grouper mettrait l'annuaire de
    /// l'espace dans l'en-tête que chacun reçoit.
    #[sqlx::test]
    async fn chaque_envoi_ne_porte_qu_une_adresse(pool: sqlx::PgPool) {
        let espion = Espion::default();
        let membres = Membres(vec![membre(ALICE, "alice"), membre(BOB, "bob")]);
        let equipes = Equipes(vec![
            equipe(T1, "Les Uns", ALICE),
            equipe(T3, "Les Trois", BOB),
        ]);

        expedier(
            &pool,
            &espion,
            &journee(vec![]),
            "2026-09-10",
            &equipes,
            &membres,
        )
        .await;

        let envois = espion.envois.lock().unwrap();
        assert_eq!(envois.len(), 2);
        assert!(envois.iter().all(|(to, _)| to.len() == 1));
    }
}
