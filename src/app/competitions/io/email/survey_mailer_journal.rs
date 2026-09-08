//! L'expédition qui journalise sans envoyer — **provisoire**.
//!
//! **Remplacée en unité 2** (`reponse-coach`), qui apporte le gabarit, la
//! fabrication du lien et la table d'envois. Elle existe pour que l'onglet de
//! l'organisateur soit entièrement vérifiable à l'écran dès maintenant : sans
//! elle, la première campagne n'aurait pu s'ouvrir qu'après l'unité suivante.
//!
//! Elle ne journalise **ni les adresses ni les jetons** : le premier est une
//! donnée personnelle, le second est un secret d'URL — qui le détient répond
//! (R7). Le compte et les noms d'équipe suffisent à savoir que l'expédition a
//! été demandée et pour combien de monde.

use crate::app::competitions::use_cases::presences::survey_mailer::{
    CampagneAAnnoncer, EnvoiPresence, ISurveyMailer, RapportEnvoi,
};
use async_trait::async_trait;

pub struct SurveyMailerJournal;

#[async_trait]
impl ISurveyMailer for SurveyMailerJournal {
    async fn expedier(
        &self,
        campagne: &CampagneAAnnoncer,
        envois: &[EnvoiPresence],
    ) -> RapportEnvoi {
        tracing::info!(
            journee = %campagne.round_name,
            echeance = %campagne.deadline,
            destinataires = envois.len(),
            "sondage de présence : expédition demandée, non implémentée (unité 2)"
        );
        // `envoyes: 0` et non `envois.len()` : rien n'est parti, et le prétendre
        // ferait afficher « 14 e-mails envoyés » à l'organisateur. R20 accepte
        // qu'un envoi échoue ; elle n'autorise pas à mentir sur son issue.
        RapportEnvoi {
            envoyes: 0,
            echecs: envois.len(),
        }
    }
}
