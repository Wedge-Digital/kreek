//! Ce que le lancement d'une campagne demande à l'expédition.
//!
//! # Le use case déclare son besoin, il n'écrit pas d'e-mail
//!
//! Le gabarit, la fabrication du lien et le journal d'envoi vivent dans
//! `io/email/survey_mailer.rs`. Ce trait est ce qui a permis de livrer l'onglet
//! de l'organisateur **avant** eux : saisie manuelle, tirage et validation
//! fonctionnaient sans qu'un seul e-mail soit parti.
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
//!
//! # Pourquoi un envoi porte **toutes** les équipes d'un coach
//!
//! Même raisonnement, appliqué à R1. Jusqu'à la carte 527, `expedier` recevait
//! un envoi **par équipe** ; regrouper par adresse était alors un travail que
//! l'implémentation devait penser à faire, et son oubli ne ressemblait pas à un
//! doublon : le premier `claim` du coach passe, le second rend zéro ligne, la
//! deuxième équipe n'a jamais ses boutons, et le compte rendu annonce « déjà
//! envoyé » pour une équipe qu'on vient de perdre.
//!
//! Le regroupement est donc **dans le type**, et il est fait par `envois_pour`,
//! qui a le roster sous la main. L'expédition, elle, ne sait pas quel coach
//! possède quelle équipe — et n'a pas à le savoir.

use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::presence_survey::{SurveyDeadline, SurveyToken};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::identity::ids::CoachId;
use async_trait::async_trait;

/// Ouverture ou relance. **Un seul type d'envoi, trois différences** : le
/// gabarit, le sujet, et la date qui entre dans la clé du journal.
///
/// Deux méthodes de trait auraient dupliqué le protocole `claim` / envoi /
/// `confirm` en entier pour ces trois différences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvoiKind {
    Ouverture,
    Relance,
}

/// Ce que l'e-mail annonce, commun à tous ses destinataires — **et ce que le
/// journal indexe**.
///
/// `season_id` et `round_id` ne s'affichent nulle part : ils composent la clé
/// d'idempotence avec le coach et la date. Les faire recharger par l'expédition
/// lui donnerait un accès aux dépôts qu'elle n'a pas à avoir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampagneAAnnoncer {
    pub season_id: SeasonId,
    pub round_id: MatchId,
    pub round_name: String,
    pub date_start: Option<String>,
    /// `None` sur une journée à date fixe : l'e-mail n'affiche alors pas de
    /// ligne « Clôture », et « Ouverture » devient « Se tient le ».
    pub date_end: Option<String>,
    pub deadline: SurveyDeadline,
    pub competition_name: String,
    /// Construite par le handler, qui possède `AppRoutes`. Un use case qui la
    /// composerait écrirait un chemin en dur.
    pub competition_url: String,
    pub kind: EnvoiKind,
    /// **Une entrée, jamais une lecture d'horloge.** Les use cases de ce BC
    /// reçoivent leur date, par convention explicite ; une expédition qui lirait
    /// l'heure rendrait la relance intestable — c'est précisément le jour de
    /// l'envoi qui entre dans sa clé.
    pub aujourd_hui: DateString,
}

/// Ce que le handler apporte, et que le use case ne saurait pas composer.
///
/// L'URL en particulier : les routes vivent dans la couche web, et un use case
/// qui la fabriquerait écrirait un chemin en dur — c'est ce que
/// `send_due_notifications_use_case` fait encore, et qu'on ne reproduit pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtiquettesCampagne {
    pub competition_name: String,
    pub competition_url: String,
}

impl CampagneAAnnoncer {
    /// Le seul point d'assemblage, appelé par le lancement **et** par la
    /// relance. Onze champs recopiés à deux endroits divergeraient au premier
    /// ajout.
    pub(crate) fn nouvelle(
        round: &MatchDay,
        deadline: SurveyDeadline,
        etiquettes: &EtiquettesCampagne,
        kind: EnvoiKind,
        aujourd_hui: &DateString,
    ) -> Self {
        Self {
            season_id: round.season_id.clone(),
            round_id: round.id.clone(),
            round_name: round.name.as_ref().to_string(),
            date_start: round.date_start.as_ref().map(|d| d.as_ref().to_string()),
            date_end: round.date_end.as_ref().map(|d| d.as_ref().to_string()),
            deadline,
            competition_name: etiquettes.competition_name.clone(),
            competition_url: etiquettes.competition_url.clone(),
            kind,
            aujourd_hui: aujourd_hui.clone(),
        }
    }
}

/// Une équipe à qui l'on demande sa présence, et le jeton qui porte sa réponse.
///
/// Le jeton **est** l'autorisation (R7) : c'est lui qui voyage dans l'URL, et il
/// n'a pas de durée de vie propre — sa validité se lit sur l'état de la campagne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipeAContacter {
    pub team_name: String,
    pub token: SurveyToken,
}

/// Un envoi : **un coach, un message, N paires de boutons** (R1).
///
/// `coach_id` n'apparaît pas dans l'e-mail ; c'est la clé du journal qui en a
/// besoin. Le retrouver depuis l'adresse serait une seconde façon d'identifier
/// un coach, à garder d'accord avec la première.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvoiPresence {
    pub coach_id: CoachId,
    pub email: String,
    /// Le nom nu, pour l'apostrophe. **Pas `coach_label`** : « Salut Lepandawan ·
    /// 2 équipes, » est ce qu'aurait donné le libellé du tableau, et un champ vide
    /// aurait donné « Salut , » — le défaut de l'e-mail d'ouverture parti avec
    /// « **** t'invite à participer ».
    pub coach_name: String,
    /// Jamais vide : un envoi sans équipe n'a rien à demander. C'est
    /// `envois_pour` qui le garantit, en n'en produisant pas.
    pub equipes: Vec<EquipeAContacter>,
}

/// Ce que l'expédition rapporte. Ni `echecs` ni `deja_envoyes` ne sont des
/// erreurs : ce sont des faits que le journal garde.
///
/// `deja_envoyes` est ce que la base a tranché — zéro ligne réservée signifie
/// « celui-là a déjà reçu son e-mail », et c'est le seul juge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RapportEnvoi {
    pub envoyes: usize,
    pub deja_envoyes: usize,
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
