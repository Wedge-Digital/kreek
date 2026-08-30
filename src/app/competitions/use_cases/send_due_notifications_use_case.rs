//! Le déclencheur périodique : ce qui est dû aujourd'hui part aujourd'hui.
//!
//! # `today` est une entrée, pas une lecture d'horloge
//!
//! C'est ce qui rend ce use case testable sans attendre le lendemain, et ce qui
//! permet à la CLI d'exposer une date forcée. Un use case qui appelle `now()`
//! lui-même n'est testable qu'en trichant sur l'horloge de la machine.
//!
//! # Le rapport n'est pas décoratif
//!
//! C'est lui que la CLI imprime et dont elle tire son code de sortie :
//! `failed > 0` vaut `exit(1)`, ce qui rend R1 observable dans les journaux du
//! cron. Une exécution parfaitement silencieuse et une exécution ayant perdu
//! douze e-mails ne doivent pas se ressembler.

use crate::app::competitions::domain::notification_delivery::NotificationType;
use crate::app::competitions::domain::notification_schedule::{
    due_today, fenetres, DueNotification,
};
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::io::repository::notification_delivery_repository::{
    NotificationDeliveryRepository, SeasonCandidate,
};
use crate::app::competitions::use_cases::notification_dispatch::{
    dispatch, DispatchDeps, DispatchLabels, DispatchOutcome,
};
use crate::app::competitions::use_cases::notification_recipients::SeasonContext;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use std::collections::HashMap;

#[derive(Debug)]
pub struct SendDueNotificationsCommand {
    pub today: DateString,
    /// N'écrit rien et n'envoie rien : compte seulement ce qui partirait.
    ///
    /// L'arrêt se fait **avant** la réservation, jamais entre elle et l'envoi :
    /// réserver puis ne rien expédier laisserait des lignes qui bloqueraient le
    /// vrai passage, et R9 interdit de les rejouer.
    pub dry_run: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SendDueNotificationsReport {
    pub seasons_examined: usize,
    pub notifications_due: usize,
    pub sent: usize,
    pub skipped_already_sent: usize,
    pub failed: usize,
}

pub struct CronDeps<'a> {
    pub seasons: &'a dyn ISeasonRepository,
    /// Pour le nom de l'administrateur, que les deux e-mails d'inscription
    /// nomment en toutes lettres — « **X** t'invite à participer ».
    pub competitions: &'a dyn crate::app::competitions::domain::competition_repository_port::ICompetitionRepository,
    pub match_days:
        &'a dyn crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository,
    pub journal: &'a NotificationDeliveryRepository,
    pub dispatch: DispatchDeps<'a>,
}

#[tracing::instrument(skip_all, fields(today = %cmd.today.as_ref(), dry_run = cmd.dry_run))]
pub async fn execute(
    cmd: SendDueNotificationsCommand,
    deps: &CronDeps<'_>,
) -> SendDueNotificationsReport {
    let mut rapport = SendDueNotificationsReport::default();

    for candidate in candidates(deps.journal, &cmd.today).await {
        rapport.seasons_examined += 1;
        traiter_saison(&candidate, &cmd, deps, &mut rapport).await;
    }

    tracing::info!(
        seasons = rapport.seasons_examined,
        due = rapport.notifications_due,
        sent = rapport.sent,
        skipped = rapport.skipped_already_sent,
        failed = rapport.failed,
        "cron de notifications terminé"
    );
    rapport
}

/// Les trois requêtes sont bornées par la date ; leurs résultats se recouvrent
/// — une saison peut avoir une journée qui démarre et une autre qui clôt le
/// même jour. On dédoublonne ici plutôt que dans le SQL : `due_today()` est de
/// toute façon appelée une fois par saison, et c'est elle qui décide.
async fn candidates(
    journal: &NotificationDeliveryRepository,
    today: &DateString,
) -> Vec<SeasonCandidate> {
    // Les décalages viennent du domaine, jamais recalculés ici : les requêtes
    // cherchent une journée **à la date donnée**, `due_today()` compare à
    // `today + n`. Les deux dates doivent sortir de la même source, sans quoi le
    // cron ne trouve jamais rien — sans la moindre erreur pour le signaler.
    let Some(f) = fenetres(today) else {
        tracing::error!("date du jour illisible");
        return Vec::new();
    };

    let mut par_id: HashMap<String, SeasonCandidate> = HashMap::new();
    for r in [
        journal
            .seasons_with_round_starting(f.round_eve.as_ref())
            .await,
        journal
            .seasons_with_round_closing(f.round_closing.as_ref())
            .await,
        journal
            .seasons_with_deadline(f.registration_deadline.as_ref())
            .await,
    ] {
        match r {
            Ok(v) => par_id.extend(v.into_iter().map(|c| (c.season_id.clone(), c))),
            Err(e) => tracing::error!("sélection des saisons impossible : {e}"),
        }
    }
    par_id.into_values().collect()
}

async fn traiter_saison(
    c: &SeasonCandidate,
    cmd: &SendDueNotificationsCommand,
    deps: &CronDeps<'_>,
    rapport: &mut SendDueNotificationsReport,
) {
    let (Ok(sid), Ok(space)) = (
        SeasonId::try_new(&c.season_id),
        SpaceId::try_new(&c.space_id),
    ) else {
        rapport.failed += 1;
        return;
    };

    let reglages = deps
        .seasons
        .find_notifications(&sid)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let invitations = deps.seasons.find_invitations(&sid).await.ok().flatten();
    let journees = deps
        .match_days
        .find_by_season(&c.season_id)
        .await
        .unwrap_or_default();

    let dues = due_today(&cmd.today, &journees, invitations.as_ref(), &reglages);
    rapport.notifications_due += dues.len();
    if cmd.dry_run {
        return;
    }

    let season = SeasonContext {
        space_id: &space,
        season_id: &c.season_id,
        invitations: invitations.as_ref(),
    };
    let etiquettes = etiquettes(c, &season, deps).await;
    for due in dues {
        let bilan = expedier(&due, &season, cmd, deps, &etiquettes).await;
        rapport.sent += bilan.sent;
        rapport.skipped_already_sent += bilan.skipped_already_sent;
        rapport.failed += bilan.failed;
    }
}

async fn expedier(
    due: &DueNotification,
    season: &SeasonContext<'_>,
    cmd: &SendDueNotificationsCommand,
    deps: &CronDeps<'_>,
    etiquettes: &DispatchLabels,
) -> DispatchOutcome {
    let (notification, round) = match due {
        DueNotification::RoundEve { round } => (NotificationType::RoundEve, Some(round)),
        DueNotification::RoundClosing { round } => (NotificationType::RoundClosing, Some(round)),
        DueNotification::RegistrationDeadline { .. } => {
            (NotificationType::RegistrationDeadline, None)
        }
    };
    dispatch(
        notification,
        season,
        round,
        &cmd.today,
        etiquettes,
        &deps.dispatch,
    )
    .await
}

/// Tout ce que les gabarits nomment. Chaque champ vaut une phrase visible : un
/// `String::new()` ici rend « **** t'invite à participer », ce qui est arrivé
/// entre la carte 340 et sa correction.
async fn etiquettes(
    c: &SeasonCandidate,
    season: &SeasonContext<'_>,
    deps: &CronDeps<'_>,
) -> DispatchLabels {
    let admin = CompetitionId::try_new(&c.competition_id)
        .ok()
        .map(|id| async move { deps.competitions.find_base_info(&id).await.ok().flatten() });
    let admin_name = match admin {
        Some(f) => f
            .await
            .and_then(|b| b.admin_names.first().cloned())
            .unwrap_or_default(),
        None => String::new(),
    };

    DispatchLabels {
        competition_name: c.competition_name.clone(),
        season_name: c.season_name.clone(),
        space_name: c.space_name.clone(),
        admin_name,
        competition_url: format!(
            "{}/app/{}/competitions/{}/{}",
            deps.dispatch.app_url, c.space_id, c.competition_id, c.season_id
        ),
        registration_deadline: season
            .invitations
            .and_then(|i| i.registration_deadline.clone())
            .unwrap_or_default(),
        remaining_slots: places_restantes(season, deps).await,
    }
}

/// « Il reste N places ». Sans plafond déclaré, la phrase n'a pas de valeur à
/// afficher : on rend une chaîne vide **et** le gabarit ne montre alors pas la
/// ligne — c'est mieux que d'annoncer « il reste  places ».
async fn places_restantes(season: &SeasonContext<'_>, deps: &CronDeps<'_>) -> String {
    let Some(max) = season.invitations.and_then(|i| i.max_participants) else {
        return String::new();
    };
    let inscrits = deps
        .dispatch
        .teams
        .find_enrolled_teams(season.season_id)
        .await
        .map(|v| v.len())
        .unwrap_or(0);
    max.saturating_sub(inscrits as u32).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::io::repository::competition_repository::CompetitionRepository;
    use crate::app::competitions::io::repository::match_day_repository::MatchDayRepository;
    use crate::app::competitions::io::repository::season_repository::SeasonRepository;
    use crate::app::competitions::ports::{
        ICompetitionSpaceMemberPort, ITeamInfoPort, SpaceMemberDto, TeamInfoDto,
    };
    use crate::app::shared_kernel::identity::ids::CoachId;
    use crate::common::services::email::{EmailError, IEmailService};
    use async_trait::async_trait;
    use sqlx::PgPool;
    use std::sync::Mutex;

    // ── Doublures ────────────────────────────────────────────────────────────
    //
    // Seuls les deux ports inter-BC sont doublés : `teams` et `members` vivent
    // dans d'autres BCs et leurs adapters ne s'assemblent pas ici. Tout le reste
    // — saisons, compétitions, journées, journal — est le **vrai** dépôt sur une
    // vraie base, parce que c'est précisément la couture entre le SQL et
    // `due_today()` que ces tests existent pour tenir.

    #[derive(Default)]
    struct Espion(Mutex<Vec<Vec<String>>>);

    #[async_trait]
    impl IEmailService for Espion {
        async fn send(&self, to: Vec<String>, _: String, _: String) -> Result<(), EmailError> {
            self.0.lock().unwrap().push(to);
            Ok(())
        }
    }

    struct Membres;
    #[async_trait]
    impl ICompetitionSpaceMemberPort for Membres {
        async fn list_space_members(&self, _: &SpaceId) -> Vec<SpaceMemberDto> {
            vec![SpaceMemberDto {
                coach_id: COACH.into(),
                coach_name: "Alice".into(),
                email: "alice@example.test".into(),
            }]
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

    struct Equipes;
    #[async_trait]
    impl ITeamInfoPort for Equipes {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![TeamInfoDto {
                team_id: EQUIPE.into(),
                team_name: "Les Marteaux".into(),
                coach_id: COACH.into(),
                coach_name: "Alice".into(),
                roster_name: "Humains".into(),
                logo_url: None,
            }])
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(Vec::new())
        }
        async fn find_team_enrollment(
            &self,
            _: &str,
        ) -> Result<Option<crate::app::competitions::ports::TeamEnrollmentDto>, String> {
            Ok(None)
        }
    }

    const ESPACE: &str = "01KZVCJZ35JZWQB0KNTA9JJEPX";
    const COMPET: &str = "01KZVCKDG19DXZHJA295WSJGMW";
    const SAISON: &str = "01KZVCKDG19DXZHJA295WSJGMV";
    const COACH: &str = "01KZVCKDG19DXZHJA295WSJGM1";
    const EQUIPE: &str = "01KZVCKDG19DXZHJA295WSJGT1";
    const AUJOURDHUI: &str = "2026-09-10";

    fn jour(decalage: i64) -> String {
        let d = time::Date::parse(
            AUJOURDHUI,
            time::macros::format_description!("[year]-[month]-[day]"),
        )
        .expect("date pivot")
            + time::Duration::days(decalage);
        d.format(time::macros::format_description!("[year]-[month]-[day]"))
            .expect("format")
    }

    /// Tous les réglages allumés, et pas de date limite : c'est le socle sur
    /// lequel chaque test ne change **qu'une** chose.
    async fn semer(pool: &PgPool, notifications: &str, deadline: Option<&str>) {
        sqlx::query("INSERT INTO spaces (id, space_name, space_icon_path) VALUES ($1,'Ligue','')")
            .bind(ESPACE)
            .execute(pool)
            .await
            .expect("espace");
        sqlx::query(
            "INSERT INTO competitions (id, space_id, name, logo) VALUES ($1,$2,'Coupe de Fer','')",
        )
        .bind(COMPET)
        .bind(ESPACE)
        .execute(pool)
        .await
        .expect("compétition");

        let invitations = match deadline {
            Some(d) => format!(r#"{{"access_mode":"open","registration_deadline":"{d}"}}"#),
            None => r#"{"access_mode":"open"}"#.to_string(),
        };
        sqlx::query(
            "INSERT INTO competition_seasons (id, competition_id, name, status, notifications, invitations)
             VALUES ($1,$2,'Saison 1','ready',$3::jsonb,$4::jsonb)",
        )
        .bind(SAISON)
        .bind(COMPET)
        .bind(notifications)
        .bind(invitations)
        .execute(pool)
        .await
        .expect("saison");
    }

    /// `appariement` est passé explicitement plutôt que dérivé de `id` : un
    /// ULID fait exactement 26 caractères, et toute dérivation par troncature ou
    /// suffixe produit soit un identifiant invalide, soit une collision entre
    /// deux journées voisines. Les deux se sont produites en écrivant ces tests.
    async fn semer_journee(
        pool: &PgPool,
        id: &str,
        appariement: &str,
        debut: &str,
        fin: Option<&str>,
        pos: i32,
    ) {
        sqlx::query(
            "INSERT INTO competition_match_days (id, season_id, name, day_type, date_start, date_end, position)
             VALUES ($1,$2,'Journée 1',$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(SAISON)
        .bind(if fin.is_some() { "time_frame" } else { "fixed_date" })
        .bind(debut)
        .bind(fin)
        .bind(pos)
        .execute(pool)
        .await
        .expect("journée");

        sqlx::query(
            "INSERT INTO competition_match_day_pairings (id, match_day_id, home_team_id, away_team_id)
             VALUES ($1,$2,$3,$4)",
        )
        .bind(appariement)
        .bind(id)
        .bind(EQUIPE)
        .bind("01KZVCKDG19DXZHJA295WSJGT2")
        .execute(pool)
        .await
        .expect("appariement");
    }

    const TOUT_ALLUME: &str = r#"{"registration_open":true,"round_eve":true,
                                  "round_closing":true,"registration_deadline":true}"#;

    async fn lancer(pool: &PgPool, dry_run: bool, espion: &Espion) -> SendDueNotificationsReport {
        let seasons = SeasonRepository::new(pool.clone());
        let competitions = CompetitionRepository::new(pool.clone());
        let match_days = MatchDayRepository::new(pool.clone());
        let journal = NotificationDeliveryRepository::new(pool.clone());
        let deps = CronDeps {
            seasons: &seasons,
            competitions: &competitions,
            match_days: &match_days,
            journal: &journal,
            dispatch: DispatchDeps {
                teams: &Equipes,
                members: &Membres,
                journal: &journal,
                email: espion,
                app_url: "https://kreek.example",
            },
        };
        execute(
            SendDueNotificationsCommand {
                today: DateString::try_new(AUJOURDHUI).expect("date du jour"),
                dry_run,
            },
            &deps,
        )
        .await
    }

    async fn journal_de(pool: &PgPool) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT notification_type FROM competition_notification_deliveries ORDER BY 1",
        )
        .fetch_all(pool)
        .await
        .expect("lecture du journal")
    }

    /// **Le mode d'échec silencieux nommé par le code lui-même** : le SQL
    /// cherche une journée à `today + n`, `due_today()` compare avec le même `n`
    /// tiré du domaine. Un décalage d'un jour ne casse rien, ne journalise rien,
    /// et n'envoie plus rien.
    #[sqlx::test]
    async fn une_journee_demain_declenche_round_eve_et_rien_d_autre(pool: PgPool) {
        semer(&pool, TOUT_ALLUME, None).await;
        semer_journee(
            &pool,
            "01KZVCKDG19DXZHJA295WSJGD1",
            "01KZVCKDG19DXZHJA295WSJGP1",
            &jour(1),
            None,
            0,
        )
        .await;

        let espion = Espion::default();
        let rapport = lancer(&pool, false, &espion).await;

        assert_eq!(rapport.seasons_examined, 1);
        assert_eq!(rapport.sent, 1, "un inscrit, un e-mail : {rapport:?}");
        assert_eq!(journal_de(&pool).await, vec!["round_eve"]);
    }

    /// `candidates()` dédoublonne trois requêtes qui se recouvrent. Sans ce
    /// `HashMap`, cette saison serait traitée trois fois — et le journal la
    /// protégerait du triple envoi, ce qui masquerait le défaut au lieu de le
    /// montrer. C'est `seasons_examined` qui le voit.
    #[sqlx::test]
    async fn une_saison_qui_sort_des_trois_requetes_n_est_traitee_qu_une_fois(pool: PgPool) {
        semer(&pool, TOUT_ALLUME, Some(&jour(3))).await;
        semer_journee(
            &pool,
            "01KZVCKDG19DXZHJA295WSJGD1",
            "01KZVCKDG19DXZHJA295WSJGP1",
            &jour(1),
            None,
            0,
        )
        .await;
        semer_journee(
            &pool,
            "01KZVCKDG19DXZHJA295WSJGD2",
            "01KZVCKDG19DXZHJA295WSJGP2",
            &jour(0),
            Some(&jour(2)),
            1,
        )
        .await;

        let espion = Espion::default();
        let rapport = lancer(&pool, false, &espion).await;

        assert_eq!(
            rapport.seasons_examined, 1,
            "la saison sort des trois lectures et n'est traitée qu'une fois : {rapport:?}"
        );
    }

    /// La commande qu'on lancera **en premier** sur la production. Une inversion
    /// entre le comptage et l'arrêt ferait envoyer une exécution censée ne rien
    /// faire, et R9 interdit de rejouer ce qui aurait été réservé.
    #[sqlx::test]
    async fn dry_run_compte_sans_rien_ecrire(pool: PgPool) {
        semer(&pool, TOUT_ALLUME, None).await;
        semer_journee(
            &pool,
            "01KZVCKDG19DXZHJA295WSJGD1",
            "01KZVCKDG19DXZHJA295WSJGP1",
            &jour(1),
            None,
            0,
        )
        .await;

        let espion = Espion::default();
        let rapport = lancer(&pool, true, &espion).await;

        assert_eq!(rapport.notifications_due, 1, "il compte : {rapport:?}");
        assert_eq!(rapport.sent, 0, "et n'envoie rien : {rapport:?}");
        assert!(espion.0.lock().unwrap().is_empty());
        assert!(
            journal_de(&pool).await.is_empty(),
            "aucune ligne réservée : une réservation sans envoi bloquerait le vrai passage"
        );
    }

    /// Le réglage commande. Décoché, la journée reste due au sens du calendrier
    /// mais rien ne part — c'est `due_today()` qui l'exclut, et ce test tient la
    /// chaîne complète entre la colonne JSONB et l'absence d'e-mail.
    #[sqlx::test]
    async fn un_reglage_decoche_n_envoie_rien(pool: PgPool) {
        let eteint = r#"{"registration_open":true,"round_eve":false,
                         "round_closing":true,"registration_deadline":true}"#;
        semer(&pool, eteint, None).await;
        semer_journee(
            &pool,
            "01KZVCKDG19DXZHJA295WSJGD1",
            "01KZVCKDG19DXZHJA295WSJGP1",
            &jour(1),
            None,
            0,
        )
        .await;

        let espion = Espion::default();
        let rapport = lancer(&pool, false, &espion).await;

        assert_eq!(rapport.seasons_examined, 1, "la saison est bien examinée");
        assert_eq!(rapport.sent, 0, "mais rien ne part : {rapport:?}");
        assert!(journal_de(&pool).await.is_empty());
    }

    /// Le critère de clôture de l'épic, seconde moitié : « sans qu'une seconde
    /// exécution du cron le même jour lui en envoie un second ». C'est l'index
    /// unique du journal qui le tient, pas une garde applicative.
    #[sqlx::test]
    async fn une_seconde_execution_le_meme_jour_n_envoie_pas_deux_fois(pool: PgPool) {
        semer(&pool, TOUT_ALLUME, None).await;
        semer_journee(
            &pool,
            "01KZVCKDG19DXZHJA295WSJGD1",
            "01KZVCKDG19DXZHJA295WSJGP1",
            &jour(1),
            None,
            0,
        )
        .await;

        let espion = Espion::default();
        let premier = lancer(&pool, false, &espion).await;
        assert_eq!(
            premier.sent, 1,
            "sans premier envoi, ce test ne prouve rien"
        );

        let second = lancer(&pool, false, &espion).await;
        assert_eq!(second.sent, 0, "second envoi : {second:?}");
        assert_eq!(second.skipped_already_sent, 1, "{second:?}");
        assert_eq!(espion.0.lock().unwrap().len(), 1, "un seul e-mail au total");
    }
}
