use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
use crate::app::competitions::domain::competition_season::CompetitionSeason;
use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::season_repository_port::{ISeasonRepository, SeasonRepositoryError};
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, CompetitionId, EventId, SeasonId, SpaceId};
use crate::app::shared_kernel::competition_name::CompetitionName;
use crate::lib::services::event_bus::event_bus::EventBus;

pub struct CreateDraftCompetitionCommand {
    pub space_id:   SpaceId,
    pub created_by: CoachId,
    pub name:       CompetitionName,
    pub logo:       CloudinaryImage,
    pub admin_ids:  Vec<CoachId>,
}

#[derive(Debug)]
pub enum CreateDraftCompetitionError {
    CompetitionNameAlreadyTaken,
    Database(String),
}

impl From<CompetitionRepositoryError> for CreateDraftCompetitionError {
    fn from(e: CompetitionRepositoryError) -> Self {
        match e {
            CompetitionRepositoryError::CompetitionNameAlreadyTaken => CreateDraftCompetitionError::CompetitionNameAlreadyTaken,
            CompetitionRepositoryError::CompetitionNotFound         => CreateDraftCompetitionError::Database("competition not found".into()),
            CompetitionRepositoryError::Database(msg)               => CreateDraftCompetitionError::Database(msg),
        }
    }
}

impl From<SeasonRepositoryError> for CreateDraftCompetitionError {
    fn from(e: SeasonRepositoryError) -> Self {
        CreateDraftCompetitionError::Database(e.to_string())
    }
}

pub async fn execute(
    cmd:        CreateDraftCompetitionCommand,
    repo:       &dyn ICompetitionRepository,
    season_repo: &dyn ISeasonRepository,
    bus:        &EventBus,
) -> Result<(CompetitionId, SeasonId), CreateDraftCompetitionError> {
    if repo.name_exists_in_space(&cmd.name, &cmd.space_id).await? {
        return Err(CreateDraftCompetitionError::CompetitionNameAlreadyTaken);
    }

    let competition = Competition::new(
        cmd.space_id,
        cmd.name.clone(),
        cmd.logo.clone(),
        cmd.admin_ids.clone(),
    );
    let competition_id = competition.id;

    repo.save(&competition).await?;

    let season = CompetitionSeason::new(competition_id, "Saison 1".to_string());
    let season_id = season.id;
    season_repo.save(&season).await?;

    let _ = bus.send(CompetitionsDomainEvent::CompetitionCreated {
        event_id:       EventId::new(),
        competition_id: competition.id,
        space_id:       competition.space_id,
        created_by:     cmd.created_by,
        name:           competition.name,
        logo:           competition.logo,
        admin_ids:      competition.admin_ids,
    }.to_enveloppe());

    Ok((competition_id, season_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::competitions::domain::competition_repository_port::{CompetitionBaseInfo, CompetitionRepositoryError, CompetitionSummary, ICompetitionRepository};
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_rules::CompetitionRules;
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{ISeasonRepository, SeasonBaseInfo, SeasonRepositoryError};
    use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
    use crate::lib::services::event_bus::event_bus::new_bus;

    const LOGO: &str = "https://res.cloudinary.com/demo/image/upload/sample.jpg";

    struct FakeCompetitionRepo { pub name_taken: bool }

    #[async_trait]
    impl ICompetitionRepository for FakeCompetitionRepo {
        async fn name_exists_in_space(&self, _: &CompetitionName, _: &SpaceId) -> Result<bool, CompetitionRepositoryError> {
            Ok(self.name_taken)
        }
        async fn save(&self, _: &Competition) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_by_space_id(&self, _: &SpaceId) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> { Ok(vec![]) }
        async fn find_base_info(&self, _: &CompetitionId) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> { Ok(None) }
        async fn update_base_info(&self, _: &CompetitionId, _: &CompetitionName, _: &CloudinaryImage, _: &[CoachId]) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_with_seasons(&self, _: &SpaceId) -> Result<Vec<crate::app::competitions::domain::competition_repository_port::CompetitionWithSeasons>, CompetitionRepositoryError> { Ok(vec![]) }
    }

    struct FakeSeasonRepo;

    #[async_trait]
    impl ISeasonRepository for FakeSeasonRepo {
        async fn save(&self, _: &CompetitionSeason) -> Result<(), SeasonRepositoryError> { Ok(()) }
        async fn find_latest_season_id(&self, _: &CompetitionId) -> Result<Option<SeasonId>, SeasonRepositoryError> { Ok(None) }
        async fn find_base_info(&self, _: &SeasonId) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> { Ok(None) }
        async fn find_rules(&self, _: &SeasonId) -> Result<Option<CompetitionRules>, SeasonRepositoryError> { Ok(None) }
        async fn save_rules(&self, _: &SeasonId, _: &str, _: &CompetitionRules) -> Result<(), SeasonRepositoryError> { Ok(()) }
        async fn find_structure(&self, _: &SeasonId) -> Result<Option<CompetitionStructure>, SeasonRepositoryError> { Ok(None) }
        async fn save_structure(&self, _: &SeasonId, _: &CompetitionStructure) -> Result<(), SeasonRepositoryError> { Ok(()) }
        async fn find_invitations(&self, _: &SeasonId) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError> { Ok(None) }
        async fn save_invitations(&self, _: &SeasonId, _: &CompetitionInvitations) -> Result<(), SeasonRepositoryError> { Ok(()) }
        async fn set_ready(&self, _: &SeasonId) -> Result<(), SeasonRepositoryError> { Ok(()) }
        async fn find_full(&self, _: &SeasonId) -> Result<Option<crate::app::competitions::domain::season_repository_port::SeasonFull>, SeasonRepositoryError> { Ok(None) }
    }

    fn make_cmd() -> CreateDraftCompetitionCommand {
        CreateDraftCompetitionCommand {
            space_id:   SpaceId::new(),
            created_by: CoachId::new(),
            name:       CompetitionName::try_new("Ligue Alpha").unwrap(),
            logo:       CloudinaryImage::try_new(LOGO).unwrap(),
            admin_ids:  vec![],
        }
    }

    #[tokio::test]
    async fn success_emits_competition_created_event() {
        let bus    = new_bus();
        let mut rx = bus.subscribe();
        let result = execute(make_cmd(), &FakeCompetitionRepo { name_taken: false }, &FakeSeasonRepo, &bus).await;
        assert!(result.is_ok());
        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.event_type, "CompetitionCreated");
    }

    #[tokio::test]
    async fn rejects_duplicate_name() {
        let result = execute(make_cmd(), &FakeCompetitionRepo { name_taken: true }, &FakeSeasonRepo, &new_bus()).await;
        assert!(matches!(result, Err(CreateDraftCompetitionError::CompetitionNameAlreadyTaken)));
    }
}