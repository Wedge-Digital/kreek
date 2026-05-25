use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::shared_kernel::common_types::{CoachId, CompetitionId, EventId, SpaceId};
use crate::lib::services::event_bus::event_bus::EventBus;

pub struct FinalizeCompetitionCommand {
    pub competition_id: CompetitionId,
    pub space_id:       SpaceId,
    pub finalized_by:   CoachId,
}

#[derive(Debug)]
pub enum FinalizeCompetitionError {
    CompetitionNotFound,
    Database(String),
}

impl From<CompetitionRepositoryError> for FinalizeCompetitionError {
    fn from(e: CompetitionRepositoryError) -> Self {
        match e {
            CompetitionRepositoryError::CompetitionNotFound => Self::CompetitionNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

pub async fn execute(
    cmd:  FinalizeCompetitionCommand,
    repo: &dyn ICompetitionRepository,
    bus:  &EventBus,
) -> Result<(), FinalizeCompetitionError> {
    repo.set_ready(&cmd.competition_id).await?;

    let _ = bus.send(CompetitionsDomainEvent::CompetitionReady {
        event_id:       EventId::new(),
        competition_id: cmd.competition_id,
        space_id:       cmd.space_id,
        finalized_by:   cmd.finalized_by,
    }.to_enveloppe());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::competitions::domain::competition::Competition;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_repository_port::{CompetitionBaseInfo, CompetitionSummary};
    use crate::app::competitions::domain::competition_rules::CompetitionRules;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::shared_kernel::common_types::{CloudinaryImage, SpaceId};
    use crate::app::shared_kernel::competition_name::CompetitionName;
    use crate::lib::services::event_bus::event_bus::new_bus;

    struct FakeRepo { fail: bool }

    #[async_trait]
    impl ICompetitionRepository for FakeRepo {
        async fn name_exists_in_space(&self, _: &CompetitionName, _: &SpaceId) -> Result<bool, CompetitionRepositoryError> { Ok(false) }
        async fn save(&self, _: &Competition)                                    -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_by_space_id(&self, _: &SpaceId)                           -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> { Ok(vec![]) }
        async fn save_rules(&self, _: &CompetitionId, _: &CompetitionRules)     -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_rules(&self, _: &CompetitionId)                           -> Result<Option<CompetitionRules>, CompetitionRepositoryError> { Ok(None) }
        async fn save_structure(&self, _: &CompetitionId, _: &CompetitionStructure) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_structure(&self, _: &CompetitionId)                       -> Result<Option<CompetitionStructure>, CompetitionRepositoryError> { Ok(None) }
        async fn save_invitations(&self, _: &CompetitionId, _: &CompetitionInvitations) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_invitations(&self, _: &CompetitionId)                     -> Result<Option<CompetitionInvitations>, CompetitionRepositoryError> { Ok(None) }
        async fn find_base_info(&self, _: &CompetitionId)                       -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> { Ok(None) }

        async fn update_base_info(&self, competition_id: &CompetitionId, name: &CompetitionName, logo: &CloudinaryImage, admin_ids: &[CoachId]) -> Result<(), CompetitionRepositoryError> {
            todo!()
        }

        async fn set_ready(&self, _: &CompetitionId) -> Result<(), CompetitionRepositoryError> {
            if self.fail { Err(CompetitionRepositoryError::CompetitionNotFound) } else { Ok(()) }
        }
    }

    fn make_cmd() -> FinalizeCompetitionCommand {
        FinalizeCompetitionCommand {
            competition_id: CompetitionId::new(),
            space_id:       SpaceId::new(),
            finalized_by:   CoachId::new(),
        }
    }

    #[tokio::test]
    async fn success_emits_competition_ready_event() {
        let bus    = new_bus();
        let mut rx = bus.subscribe();

        let result = execute(make_cmd(), &FakeRepo { fail: false }, &bus).await;

        assert!(result.is_ok());
        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.event_type, "CompetitionReady");
    }

    #[tokio::test]
    async fn not_found_returns_error() {
        let result = execute(make_cmd(), &FakeRepo { fail: true }, &new_bus()).await;
        assert!(matches!(result, Err(FinalizeCompetitionError::CompetitionNotFound)));
    }
}