use crate::app::competitions::domain::competition::Competition;
use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, CompetitionId, EventId, SpaceId};
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

pub async fn execute(
    cmd:  CreateDraftCompetitionCommand,
    repo: &dyn ICompetitionRepository,
    bus:  &EventBus,
) -> Result<CompetitionId, CreateDraftCompetitionError> {
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

    let _ = bus.send(CompetitionsDomainEvent::CompetitionCreated {
        event_id:       EventId::new(),
        competition_id: competition.id,
        space_id:       competition.space_id,
        created_by:     cmd.created_by,
        name:           competition.name,
        logo:           competition.logo,
        admin_ids:      competition.admin_ids,
    }.to_enveloppe());

    Ok(competition_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_repository_port::{CompetitionBaseInfo, CompetitionRepositoryError, CompetitionSummary, ICompetitionRepository};
    use crate::app::competitions::domain::competition_rules::CompetitionRules;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
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
        async fn save_rules(&self, _: &CompetitionId, _: &CompetitionRules) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_rules(&self, _: &CompetitionId) -> Result<Option<CompetitionRules>, CompetitionRepositoryError> { Ok(None) }
        async fn save_structure(&self, _: &CompetitionId, _: &CompetitionStructure) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_structure(&self, _: &CompetitionId) -> Result<Option<CompetitionStructure>, CompetitionRepositoryError> { Ok(None) }
        async fn save_invitations(&self, _: &CompetitionId, _: &CompetitionInvitations) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_invitations(&self, _: &CompetitionId) -> Result<Option<CompetitionInvitations>, CompetitionRepositoryError> { Ok(None) }
        async fn find_base_info(&self, _: &CompetitionId) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> { Ok(None) }
        async fn update_base_info(&self, _: &CompetitionId, _: &CompetitionName, _: &CloudinaryImage, _: &[CoachId]) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn set_ready(&self, _: &CompetitionId) -> Result<(), CompetitionRepositoryError> { Ok(()) }
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
        let result = execute(make_cmd(), &FakeCompetitionRepo { name_taken: false }, &bus).await;
        assert!(result.is_ok());
        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.event_type, "CompetitionCreated");
    }

    #[tokio::test]
    async fn rejects_duplicate_name() {
        let result = execute(make_cmd(), &FakeCompetitionRepo { name_taken: true }, &new_bus()).await;
        assert!(matches!(result, Err(CreateDraftCompetitionError::CompetitionNameAlreadyTaken)));
    }
}
