//! Ce que le lancement d'une campagne demande à l'expédition.
//!
//! # Le use case déclare son besoin, il n'écrit pas d'e-mail
//!
//! Le gabarit, la fabrication du lien et le journal d'envoi sont le contenu de
//! l'unité `reponse-coach`. Ce trait est ce qui permet de livrer l'onglet de
//! l'organisateur **avant** eux : saisie manuelle, tirage et validation
//! fonctionnent sans qu'un seul e-mail soit parti.
//!
//! # Pourquoi `expedier` ne rend pas de `Result`
//!
//! R20 : la campagne est persistée avant l'expédition, et un échec d'envoi est
//! **journalisé, pas propagé** — un serveur de messagerie indisponible ne doit
//! pas bloquer une fonction qui reste utilisable sans lui.
//!
//! Avec un `Result`, tenir R20 dépendrait de la discipline de chaque appelant :
//! un `?` de trop, et la panne du serveur SMTP annule le lancement. Avec un
//! `RapportEnvoi`, **il n'y a rien à propager** — la règle devient structurelle
//! au lieu d'être une consigne. C'est le raisonnement de « pas d'implémentation
//! par défaut qui bouclerait sur `save_pairing` » : un verrou qui se laisse
//! oublier n'est pas un verrou.

use crate::app::competitions::domain::presence_survey::{SurveyDeadline, SurveyToken};
use async_trait::async_trait;

/// Ce que l'e-mail annonce, commun à tous ses destinataires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampagneAAnnoncer {
    pub round_name: String,
    pub deadline: SurveyDeadline,
}

/// Un envoi : à qui, pour quelle équipe, et le jeton qui fait le lien.
///
/// Le jeton **est** l'autorisation (R7) : c'est lui qui voyage dans l'URL, et il
/// n'a pas de durée de vie propre — sa validité se lit sur l'état de la campagne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvoiPresence {
    pub email: String,
    pub coach_label: String,
    pub team_name: String,
    pub token: SurveyToken,
}

/// Ce que l'expédition rapporte. `echecs` n'est pas une erreur : c'est un fait
/// que l'écran annonce et que le journal garde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RapportEnvoi {
    pub envoyes: usize,
    pub echecs: usize,
}

#[async_trait]
pub trait ISurveyMailer: Send + Sync {
    async fn expedier(
        &self,
        campagne: &CampagneAAnnoncer,
        envois: &[EnvoiPresence],
    ) -> RapportEnvoi;
}
