use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_season::CompetitionSeason;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonBaseInfo, SeasonFull, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use async_trait::async_trait;

pub struct FakeSeasonRepository;

#[async_trait]
impl ISeasonRepository for FakeSeasonRepository {
    /// Les doublures ne portent pas d'espace : ce sont les tests de handler,
    /// sur une vraie base, qui exercent le contrôle d'appartenance.
    async fn find_space_id(&self, _: &SeasonId) -> Result<Option<String>, SeasonRepositoryError> {
        Ok(None)
    }

    async fn save(&self, _: &CompetitionSeason) -> Result<(), SeasonRepositoryError> {
        Ok(())
    }
    async fn find_latest_season_id(
        &self,
        _: &CompetitionId,
    ) -> Result<Option<SeasonId>, SeasonRepositoryError> {
        Ok(None)
    }
    async fn find_base_info(
        &self,
        _: &SeasonId,
    ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> {
        Ok(None)
    }
    async fn find_rules(
        &self,
        _: &SeasonId,
    ) -> Result<Option<CompetitionRules>, SeasonRepositoryError> {
        Ok(None)
    }
    async fn save_rules(
        &self,
        _: &SeasonId,
        _: &str,
        _: &CompetitionRules,
    ) -> Result<(), SeasonRepositoryError> {
        Ok(())
    }
    async fn find_structure(
        &self,
        _: &SeasonId,
    ) -> Result<Option<CompetitionStructure>, SeasonRepositoryError> {
        Ok(None)
    }
    async fn save_structure(
        &self,
        _: &SeasonId,
        _: &CompetitionStructure,
    ) -> Result<(), SeasonRepositoryError> {
        Ok(())
    }
    async fn find_invitations(
        &self,
        _: &SeasonId,
    ) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError> {
        Ok(None)
    }
    async fn save_invitations(
        &self,
        _: &SeasonId,
        _: &CompetitionInvitations,
    ) -> Result<(), SeasonRepositoryError> {
        Ok(())
    }
    async fn find_notifications(
        &self,
        _: &SeasonId,
    ) -> Result<Option<CompetitionNotifications>, SeasonRepositoryError> {
        Ok(None)
    }
    async fn save_notifications(
        &self,
        _: &SeasonId,
        _: &CompetitionNotifications,
    ) -> Result<(), SeasonRepositoryError> {
        Ok(())
    }
    async fn set_ready(&self, _: &SeasonId) -> Result<(), SeasonRepositoryError> {
        Ok(())
    }
    async fn find_full(&self, _: &SeasonId) -> Result<Option<SeasonFull>, SeasonRepositoryError> {
        Ok(None)
    }
}
