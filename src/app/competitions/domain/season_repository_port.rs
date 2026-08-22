use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_season::CompetitionSeason;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use async_trait::async_trait;

pub struct SeasonBaseInfo {
    pub name: String,
}

pub struct SeasonFull {
    pub season_id: String,
    pub season_name: String,
    pub status: String,
    pub competition_id: String,
    pub competition_name: String,
    pub rules: Option<CompetitionRules>,
    pub structure: Option<CompetitionStructure>,
}

#[derive(Debug)]
pub enum SeasonRepositoryError {
    SeasonNotFound,
    Database(String),
}

impl std::fmt::Display for SeasonRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeasonRepositoryError::SeasonNotFound => write!(f, "season not found"),
            SeasonRepositoryError::Database(e) => write!(f, "database error: {}", e),
        }
    }
}

#[async_trait]
pub trait ISeasonRepository: Send + Sync {
    async fn save(&self, season: &CompetitionSeason) -> Result<(), SeasonRepositoryError>;
    async fn find_latest_season_id(
        &self,
        competition_id: &CompetitionId,
    ) -> Result<Option<SeasonId>, SeasonRepositoryError>;
    /// L'espace auquel appartient cette saison, ou `None` si elle n'existe pas
    /// (carte 324).
    ///
    /// Une saison n'a **pas d'espace en propre** : elle en hérite de sa
    /// compétition. Le saut se fait en une jointure plutôt qu'en dénormalisant
    /// une colonne `space_id`, qui créerait une seconde source de vérité —
    /// vouée à diverger, comme la carte 313 l'a rappelé.
    async fn find_space_id(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<String>, SeasonRepositoryError>;

    async fn find_base_info(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError>;
    async fn find_rules(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionRules>, SeasonRepositoryError>;
    async fn save_rules(
        &self,
        season_id: &SeasonId,
        name: &str,
        rules: &CompetitionRules,
    ) -> Result<(), SeasonRepositoryError>;
    async fn find_structure(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionStructure>, SeasonRepositoryError>;
    async fn save_structure(
        &self,
        season_id: &SeasonId,
        structure: &CompetitionStructure,
    ) -> Result<(), SeasonRepositoryError>;
    async fn find_invitations(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError>;
    /// Écrit les deux colonnes en une instruction. L'étape 4 du magicien
    /// enregistre d'un bloc — invitations **et** réglages de notification —, et
    /// deux appels laisseraient une fenêtre où l'un a réussi et l'autre non.
    async fn save_invitations(
        &self,
        season_id: &SeasonId,
        invitations: &CompetitionInvitations,
        notifications: &CompetitionNotifications,
    ) -> Result<(), SeasonRepositoryError>;
    /// `None` quand la colonne est `NULL` — une saison qui n'a jamais été
    /// réglée. L'appelant y applique le défaut du domaine, qui vaut allumé.
    async fn find_notifications(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionNotifications>, SeasonRepositoryError>;
    async fn save_notifications(
        &self,
        season_id: &SeasonId,
        notifications: &CompetitionNotifications,
    ) -> Result<(), SeasonRepositoryError>;
    async fn set_ready(&self, season_id: &SeasonId) -> Result<(), SeasonRepositoryError>;
    async fn find_full(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<SeasonFull>, SeasonRepositoryError>;
}
