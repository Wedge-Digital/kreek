//! L'expédition du sondage de présence — réserver, rendre, envoyer, confirmer.
//!
//! Remplace `SurveyMailerJournal`, qui journalisait sans envoyer et comptait
//! tous ses destinataires en échecs. Le protocole est celui de
//! `notification_dispatch::traiter`, et pour les mêmes raisons.
//!
//! # `claim` avant l'envoi, jamais après
//!
//! C'est l'index unique de `notification_deliveries` qui départage deux appels
//! concurrents : zéro ligne réservée signifie « déjà envoyé ». **La base
//! tranche, le code n'arbitre rien** — un `SELECT` puis un `INSERT` laisseraient
//! passer les deux moitiés d'un double clic.
//!
//! Entre la réservation et la confirmation, la ligne existe avec `sent_at` à
//! `NULL`. Si l'envoi échoue, elle **reste** dans cet état : un échec constaté,
//! que R20 veut journalisé et qu'une relance du lendemain rattrape.
//!
//! # Aucune transaction n'enveloppe les quatre étapes
//!
//! Délibéré, et pour la raison qu'écrit déjà `notification_dispatch` : un appel
//! réseau se produit entre la réservation et la confirmation, et tenir une
//! transaction ouverte pendant un aller-retour SMTP est exactement ce qu'on
//! cherche à éviter. La règle de transaction unique du `CLAUDE.md` vise les
//! projections event-sourcées, pas ceci.
//!
//! # Les deux `target_date`, et le piège qu'elles referment
//!
//! | Envoi | `target_date` | Ce que ça garantit |
//! |---|---|---|
//! | ouverture | l'**échéance** de la campagne | un seul envoi par campagne et par coach |
//! | relance | le **jour de l'envoi** | une par jour : mardi puis jeudi, mais pas deux fois mardi |
//!
//! Prendre l'échéance pour la relance aussi l'aurait bloquée **définitivement dès
//! la seconde**, sans rien dire : l'organisateur aurait cliqué « Relancer », lu
//! « 0 envoyé », et cherché la panne dans le serveur de messagerie.
//!
//! # Ce que cette implémentation ne fait pas
//!
//! Elle ne regroupe pas par coach — elle reçoit déjà un `EnvoiPresence` par
//! coach, avec toutes ses équipes (R1). Le regroupement est dans `envois_pour`,
//! qui a le roster ; le mettre ici aurait demandé de savoir quel coach possède
//! quelle équipe, et l'oubli se serait vu comme un « déjà envoyé » sur une
//! équipe silencieusement perdue.

use crate::app::competitions::domain::notification_delivery::{DeliveryKey, NotificationType};
use crate::app::competitions::io::email::notification_emails::{
    EquipeLigneVm, PresenceReminderEmail, PresenceSurveyEmail,
};
use crate::app::competitions::io::repository::notification_delivery_repository::NotificationDeliveryRepository;
use crate::app::competitions::use_cases::presences::survey_mailer::{
    CampagneAAnnoncer, EnvoiKind, EnvoiPresence, ISurveyMailer, RapportEnvoi,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::common::services::email::IEmailService;
use askama::Template;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

pub struct SurveyMailer {
    journal: NotificationDeliveryRepository,
    email: Arc<dyn IEmailService>,
    /// L'URL publique, schéma compris, normalisée une fois par
    /// `AppConfig::app_url()`. Personne ne recolle plus de schéma.
    app_url: String,
}

#[async_trait]
impl ISurveyMailer for SurveyMailer {
    async fn expedier(
        &self,
        campagne: &CampagneAAnnoncer,
        envois: &[EnvoiPresence],
    ) -> RapportEnvoi {
        let mut bilan = RapportEnvoi::default();
        for envoi in envois {
            self.traiter(campagne, envoi, &mut bilan).await;
        }
        tracing::info!(
            journee = %campagne.round_name,
            envoi = ?campagne.kind,
            envoyes = bilan.envoyes,
            deja_envoyes = bilan.deja_envoyes,
            echecs = bilan.echecs,
            "sondage de présence : expédition terminée"
        );
        bilan
    }
}

impl SurveyMailer {
    pub fn new(pool: &PgPool, email: Arc<dyn IEmailService>, app_url: String) -> Self {
        Self {
            journal: NotificationDeliveryRepository::new(pool.clone()),
            email,
            app_url,
        }
    }

    /// Un destinataire, du créneau réservé à la confirmation.
    ///
    /// Un échec unitaire **n'interrompt pas la boucle** : une adresse invalide ne
    /// doit pas priver les treize autres coachs de leur e-mail.
    async fn traiter(
        &self,
        campagne: &CampagneAAnnoncer,
        envoi: &EnvoiPresence,
        bilan: &mut RapportEnvoi,
    ) {
        let Some(cle) = cle(campagne, envoi) else {
            tracing::error!(coach = %envoi.coach_id, "clé d'envoi inconstructible");
            bilan.echecs += 1;
            return;
        };
        if !self.reserver(&cle, envoi, bilan).await {
            return;
        }
        let (sujet, html) = self.rendre(campagne, envoi);
        self.envoyer(&cle, envoi, sujet, html, bilan).await;
    }

    /// `true` quand le créneau est à nous. `false` couvre les deux refus — déjà
    /// envoyé, ou base indisponible — chacun compté dans sa colonne.
    async fn reserver(
        &self,
        cle: &DeliveryKey,
        envoi: &EnvoiPresence,
        bilan: &mut RapportEnvoi,
    ) -> bool {
        match self.journal.claim(cle).await {
            Ok(true) => true,
            Ok(false) => {
                bilan.deja_envoyes += 1;
                false
            }
            Err(e) => {
                tracing::error!(coach = %envoi.coach_id, "réservation impossible : {e}");
                bilan.echecs += 1;
                false
            }
        }
    }

    async fn envoyer(
        &self,
        cle: &DeliveryKey,
        envoi: &EnvoiPresence,
        sujet: String,
        html: String,
        bilan: &mut RapportEnvoi,
    ) {
        // L'appel d'expédition tient sur **une** ligne, et c'est délibéré : l'axe
        // 12 ne lit que la ligne de l'appel et celle qui la précède, tandis que
        // `cargo fmt` répartit volontiers celui-ci sur quatre lignes — le
        // marqueur se retrouve alors hors de portée, et la vérification échoue
        // sur un envoi d'e-mail qui n'a jamais concerné le bus.
        //
        // L'axe ne dépouille pas non plus les commentaires : écrire le nom de la
        // méthode dans cette explication suffisait à la faire échouer sur elle.
        let destinataire = vec![envoi.email.clone()];
        // arch:ok envoi d'e-mail, pas une émission d'évènement — le bus n'est pas en cause
        match self.email.send(destinataire, sujet, html).await {
            Ok(()) => {
                // Une confirmation en échec laisse la ligne à `NULL` : l'e-mail
                // est parti, on ne peut pas l'attester, et le prochain passage
                // ne le rejouera pas pour autant — la ligne existe.
                if let Err(e) = self.journal.confirm(cle).await {
                    tracing::error!(coach = %envoi.coach_id, "confirmation impossible : {e}");
                }
                bilan.envoyes += 1;
            }
            Err(e) => {
                tracing::warn!(coach = %envoi.coach_id, "envoi en échec : {e}");
                bilan.echecs += 1;
            }
        }
    }

    fn rendre(&self, campagne: &CampagneAAnnoncer, envoi: &EnvoiPresence) -> (String, String) {
        let equipes = self.lignes(envoi);
        match campagne.kind {
            EnvoiKind::Ouverture => (
                format!(
                    "Seras-tu là pour {} ? — {}",
                    campagne.round_name, campagne.competition_name
                ),
                corps(sondage(&self.app_url, campagne, envoi, equipes).render()),
            ),
            EnvoiKind::Relance => (
                format!("Il te reste peu de temps pour {}", campagne.round_name),
                corps(relance(&self.app_url, campagne, envoi, equipes).render()),
            ),
        }
    }

    /// Une paire de liens par équipe. **Les URL se fabriquent ici et nulle part
    /// ailleurs** : le gabarit les reçoit construites, sans quoi la forme du lien
    /// vivrait dans du HTML d'e-mail, hors de portée de tout test.
    fn lignes(&self, envoi: &EnvoiPresence) -> Vec<EquipeLigneVm> {
        let routes = AppRoutes::default();
        envoi
            .equipes
            .iter()
            .map(|e| {
                let jeton = e.token.to_string();
                EquipeLigneVm {
                    team_name: e.team_name.clone(),
                    yes_url: format!(
                        "{}{}",
                        self.app_url,
                        routes.competitions.presence_oui(&jeton)
                    ),
                    no_url: format!(
                        "{}{}",
                        self.app_url,
                        routes.competitions.presence_non(&jeton)
                    ),
                }
            })
            .collect()
    }
}

/// Un rendu en échec ne doit pas faire tomber l'expédition des suivants — mais il
/// ne doit pas non plus partir en corps vide et silencieux. Askama n'échoue ici
/// que sur une erreur de formatage, donc jamais en pratique ; la trace est là pour
/// le jour où « jamais » arrive.
fn corps(rendu: askama::Result<String>) -> String {
    match rendu {
        Ok(html) => html,
        Err(e) => {
            tracing::error!("rendu du gabarit de présence impossible : {e}");
            String::new()
        }
    }
}

fn sondage(
    app_url: &str,
    campagne: &CampagneAAnnoncer,
    envoi: &EnvoiPresence,
    equipes: Vec<EquipeLigneVm>,
) -> PresenceSurveyEmail {
    PresenceSurveyEmail {
        app_url: app_url.to_string(),
        coach_name: envoi.coach_name.clone(),
        competition_name: campagne.competition_name.clone(),
        competition_url: campagne.competition_url.clone(),
        round_name: campagne.round_name.clone(),
        date_start: campagne.date_start.clone().unwrap_or_default(),
        date_end: campagne.date_end.clone(),
        deadline: campagne.deadline.as_ref().to_string(),
        equipes,
    }
}

fn relance(
    app_url: &str,
    campagne: &CampagneAAnnoncer,
    envoi: &EnvoiPresence,
    equipes: Vec<EquipeLigneVm>,
) -> PresenceReminderEmail {
    PresenceReminderEmail {
        app_url: app_url.to_string(),
        coach_name: envoi.coach_name.clone(),
        competition_name: campagne.competition_name.clone(),
        competition_url: campagne.competition_url.clone(),
        round_name: campagne.round_name.clone(),
        date_start: campagne.date_start.clone().unwrap_or_default(),
        date_end: campagne.date_end.clone(),
        deadline: campagne.deadline.as_ref().to_string(),
        equipes,
    }
}

fn cle(campagne: &CampagneAAnnoncer, envoi: &EnvoiPresence) -> Option<DeliveryKey> {
    Some(DeliveryKey {
        notification_type: match campagne.kind {
            EnvoiKind::Ouverture => NotificationType::PresenceSurvey,
            EnvoiKind::Relance => NotificationType::PresenceReminder,
        },
        season_id: campagne.season_id.clone(),
        round_id: Some(campagne.round_id.clone()),
        target_date: target_date(campagne)?,
        coach_id: envoi.coach_id,
    })
}

/// L'échéance pour l'ouverture, le jour de l'envoi pour la relance — cf. l'en-tête
/// de ce module.
fn target_date(campagne: &CampagneAAnnoncer) -> Option<DateString> {
    match campagne.kind {
        EnvoiKind::Ouverture => DateString::try_new(campagne.deadline.as_ref().to_string()).ok(),
        EnvoiKind::Relance => Some(campagne.aujourd_hui.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::presence_survey::{SurveyDeadline, SurveyToken};
    use crate::app::competitions::use_cases::presences::survey_mailer::EquipeAContacter;
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
    use crate::app::shared_kernel::identity::ids::CoachId;
    use crate::common::services::email::EmailError;
    use std::sync::Mutex;

    // ── L'espion ─────────────────────────────────────────────────────────────

    #[derive(Default)]
    struct Espion {
        envois: Mutex<Vec<(Vec<String>, String, String)>>,
        echoue: bool,
    }

    #[async_trait]
    impl IEmailService for Espion {
        async fn send(
            &self,
            to: Vec<String>,
            sujet: String,
            html: String,
        ) -> Result<(), EmailError> {
            self.envois.lock().unwrap().push((to, sujet, html));
            if self.echoue {
                return Err(EmailError::Network("panne simulée".into()));
            }
            Ok(())
        }
    }

    impl Espion {
        fn corps(&self) -> Vec<String> {
            self.envois
                .lock()
                .unwrap()
                .iter()
                .map(|(_, _, h)| h.clone())
                .collect()
        }
    }

    // ── Fixtures ─────────────────────────────────────────────────────────────

    const APP: &str = "https://kreek.example";

    fn campagne(kind: EnvoiKind, aujourd_hui: &str) -> CampagneAAnnoncer {
        CampagneAAnnoncer {
            season_id: SeasonId::new(),
            round_id: MatchId::new(),
            round_name: "Journée 4".into(),
            date_start: Some("12/10/2026".into()),
            date_end: Some("19/10/2026".into()),
            deadline: SurveyDeadline::try_new("2026-10-09".to_string()).unwrap(),
            competition_name: "Ligue de Fer".into(),
            competition_url: "https://kreek.example/app/e/competitions/c/s".into(),
            kind,
            aujourd_hui: DateString::try_new(aujourd_hui.to_string()).unwrap(),
        }
    }

    fn envoi(nom: &str, equipes: &[(&str, SurveyToken)]) -> EnvoiPresence {
        EnvoiPresence {
            coach_id: CoachId::new(),
            email: format!("{nom}@example.test"),
            coach_name: nom.to_string(),
            equipes: equipes
                .iter()
                .map(|(n, t)| EquipeAContacter {
                    team_name: n.to_string(),
                    token: *t,
                })
                .collect(),
        }
    }

    fn mailer(pool: &sqlx::PgPool, espion: Arc<Espion>) -> SurveyMailer {
        SurveyMailer::new(pool, espion, APP.to_string())
    }

    async fn lignes(pool: &sqlx::PgPool) -> Vec<(String, Option<time::OffsetDateTime>)> {
        sqlx::query_as("SELECT notification_type, sent_at FROM competition_notification_deliveries")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    // ── R1 ───────────────────────────────────────────────────────────────────

    /// **Le cas qui donne son sens au regroupement.** Un coach à deux équipes
    /// reçoit un message et quatre liens ; sans regroupement, le second `claim`
    /// rendrait zéro ligne et sa deuxième équipe n'aurait jamais ses boutons.
    #[sqlx::test]
    async fn un_coach_a_deux_equipes_recoit_un_seul_email_a_quatre_liens(pool: sqlx::PgPool) {
        let espion = Arc::new(Espion::default());
        let (t1, t2) = (SurveyToken::new(), SurveyToken::new());

        let bilan = mailer(&pool, espion.clone())
            .expedier(
                &campagne(EnvoiKind::Ouverture, "2026-10-01"),
                &[envoi("alice", &[("Les Crocs", t1), ("Les Choux", t2)])],
            )
            .await;

        assert_eq!(bilan.envoyes, 1, "un coach, un message");
        assert_eq!(espion.corps().len(), 1);
        let html = &espion.corps()[0];
        for url in [
            format!("{APP}/presence/{t1}/oui"),
            format!("{APP}/presence/{t1}/non"),
            format!("{APP}/presence/{t2}/oui"),
            format!("{APP}/presence/{t2}/non"),
        ] {
            assert!(html.contains(&url), "lien absent : {url}");
        }
    }

    /// Le défaut de l'e-mail d'ouverture parti avec « **** t'invite à
    /// participer » : une variable vide se rend sans rien dire.
    #[sqlx::test]
    async fn l_e_mail_apostrophe_le_coach_par_son_nom(pool: sqlx::PgPool) {
        let espion = Arc::new(Espion::default());

        mailer(&pool, espion.clone())
            .expedier(
                &campagne(EnvoiKind::Ouverture, "2026-10-01"),
                &[envoi("alice", &[("Les Crocs", SurveyToken::new())])],
            )
            .await;

        assert!(espion.corps()[0].contains("Salut <strong>alice</strong>"));
    }

    /// Trente adresses dans un même en-tête, c'est l'annuaire de l'espace
    /// distribué à tout le monde.
    #[sqlx::test]
    async fn chaque_envoi_ne_porte_qu_une_adresse(pool: sqlx::PgPool) {
        let espion = Arc::new(Espion::default());

        mailer(&pool, espion.clone())
            .expedier(
                &campagne(EnvoiKind::Ouverture, "2026-10-01"),
                &[
                    envoi("alice", &[("Les Uns", SurveyToken::new())]),
                    envoi("bob", &[("Les Deux", SurveyToken::new())]),
                ],
            )
            .await;

        let envois = espion.envois.lock().unwrap();
        assert_eq!(envois.len(), 2);
        assert!(envois.iter().all(|(to, _, _)| to.len() == 1));
    }

    // ── Le journal ───────────────────────────────────────────────────────────

    /// La base tranche, pas le code : le second appel ne rend rien parce que
    /// l'index unique refuse la ligne.
    #[sqlx::test]
    async fn un_second_lancement_ne_renvoie_rien_et_le_dit(pool: sqlx::PgPool) {
        let espion = Arc::new(Espion::default());
        let expedition = mailer(&pool, espion.clone());
        let c = campagne(EnvoiKind::Ouverture, "2026-10-01");
        let e = [envoi("alice", &[("Les Uns", SurveyToken::new())])];

        let un = expedition.expedier(&c, &e).await;
        let deux = expedition.expedier(&c, &e).await;

        assert_eq!(un.envoyes, 1);
        assert_eq!(deux.envoyes, 0);
        assert_eq!(deux.deja_envoyes, 1);
        assert_eq!(espion.corps().len(), 1, "un seul e-mail est parti");
    }

    /// R20 — l'échec est **constaté**, pas propagé : la ligne reste à `NULL`, et
    /// le coach suivant reçoit quand même son e-mail.
    #[sqlx::test]
    async fn un_envoi_en_echec_laisse_sa_ligne_sans_date_et_n_arrete_pas_la_boucle(
        pool: sqlx::PgPool,
    ) {
        let espion = Arc::new(Espion {
            echoue: true,
            ..Default::default()
        });

        let bilan = mailer(&pool, espion.clone())
            .expedier(
                &campagne(EnvoiKind::Ouverture, "2026-10-01"),
                &[
                    envoi("alice", &[("Les Uns", SurveyToken::new())]),
                    envoi("bob", &[("Les Deux", SurveyToken::new())]),
                ],
            )
            .await;

        assert_eq!(bilan.echecs, 2);
        assert_eq!(bilan.envoyes, 0);
        assert_eq!(
            espion.corps().len(),
            2,
            "le premier échec n'a pas privé le second de sa tentative"
        );
        let lignes = lignes(&pool).await;
        assert_eq!(lignes.len(), 2);
        assert!(lignes.iter().all(|(_, sent)| sent.is_none()));
    }

    /// **Ce que la seconde variante de `NotificationType` achète.** Avec une
    /// seule, la relance du jour de l'ouverture aurait partagé sa clé et se
    /// serait tue.
    #[sqlx::test]
    async fn une_relance_le_jour_de_l_ouverture_part_quand_meme(pool: sqlx::PgPool) {
        let espion = Arc::new(Espion::default());
        let expedition = mailer(&pool, espion.clone());
        let e = [envoi("alice", &[("Les Uns", SurveyToken::new())])];

        let ouverture = campagne(EnvoiKind::Ouverture, "2026-10-01");
        let mut relance = ouverture.clone();
        relance.kind = EnvoiKind::Relance;

        let un = expedition.expedier(&ouverture, &e).await;
        let deux = expedition.expedier(&relance, &e).await;

        assert_eq!(un.envoyes, 1);
        assert_eq!(deux.envoyes, 1, "la relance a sa propre clé");
        assert_eq!(espion.corps().len(), 2);
    }

    /// La clé de la relance porte le **jour de l'envoi** : deux fois mardi ne
    /// part qu'une fois, mardi puis jeudi part deux fois. Prendre l'échéance
    /// l'aurait bloquée définitivement dès la seconde.
    #[sqlx::test]
    async fn la_relance_se_rejoue_un_autre_jour_mais_pas_deux_fois_le_meme(pool: sqlx::PgPool) {
        let espion = Arc::new(Espion::default());
        let expedition = mailer(&pool, espion.clone());
        let e = [envoi("alice", &[("Les Uns", SurveyToken::new())])];

        let mardi = campagne(EnvoiKind::Relance, "2026-10-06");
        let mut jeudi = mardi.clone();
        jeudi.aujourd_hui = DateString::try_new("2026-10-08".to_string()).unwrap();

        let a = expedition.expedier(&mardi, &e).await;
        let b = expedition.expedier(&mardi, &e).await;
        let c = expedition.expedier(&jeudi, &e).await;

        assert_eq!((a.envoyes, b.envoyes, c.envoyes), (1, 0, 1));
        assert_eq!(b.deja_envoyes, 1);
        assert_eq!(espion.corps().len(), 2);
    }

    /// Les deux gabarits ne se confondent pas, et le sujet non plus.
    #[sqlx::test]
    async fn l_ouverture_et_la_relance_ne_disent_pas_la_meme_chose(pool: sqlx::PgPool) {
        let espion = Arc::new(Espion::default());
        let expedition = mailer(&pool, espion.clone());
        let e = [envoi("alice", &[("Les Uns", SurveyToken::new())])];

        expedition
            .expedier(&campagne(EnvoiKind::Ouverture, "2026-10-01"), &e)
            .await;
        expedition
            .expedier(&campagne(EnvoiKind::Relance, "2026-10-06"), &e)
            .await;

        let envois = espion.envois.lock().unwrap();
        assert_eq!(envois[0].1, "Seras-tu là pour Journée 4 ? — Ligue de Fer");
        assert_eq!(envois[1].1, "Il te reste peu de temps pour Journée 4");
        assert!(envois[0].2.contains("Seras-tu là pour Journée 4 ?"));
        assert!(envois[1]
            .2
            .contains("Tu n'as pas encore dit si tu serais là"));
    }
}
