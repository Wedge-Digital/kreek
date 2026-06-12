use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::common_types::{CoachId, CompetitionId, EventId, SeasonId, SpaceId};
use crate::common::services::event_bus::event_bus::EventBus;

pub struct FinalizeCompetitionCommand {
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub space_id: SpaceId,
    pub finalized_by: CoachId,
}

#[derive(Debug)]
pub enum FinalizeCompetitionError {
    SeasonNotFound,
    Database(String),
}

impl From<SeasonRepositoryError> for FinalizeCompetitionError {
    fn from(e: SeasonRepositoryError) -> Self {
        match e {
            SeasonRepositoryError::SeasonNotFound => Self::SeasonNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

pub async fn execute(
    cmd: FinalizeCompetitionCommand,
    season_repo: &dyn ISeasonRepository,
    bus: &EventBus,
) -> Result<(), FinalizeCompetitionError> {
    season_repo.set_ready(&cmd.season_id).await?;

    let _ = bus.send(
        CompetitionsDomainEvent::CompetitionReady {
            event_id: EventId::new(),
            competition_id: cmd.competition_id,
            space_id: cmd.space_id,
            finalized_by: cmd.finalized_by,
        }
        .to_enveloppe(),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_rules::CompetitionRules;
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{
        SeasonBaseInfo, SeasonRepositoryError,
    };
    use crate::app::shared_kernel::common_types::{CompetitionId, SpaceId};
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;

    struct FakeRepo {
        fail: bool,
    }

    #[async_trait]
    impl ISeasonRepository for FakeRepo {
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
        async fn set_ready(&self, _: &SeasonId) -> Result<(), SeasonRepositoryError> {
            if self.fail {
                Err(SeasonRepositoryError::SeasonNotFound)
            } else {
                Ok(())
            }
        }
        async fn find_full(
            &self,
            _: &SeasonId,
        ) -> Result<
            Option<crate::app::competitions::domain::season_repository_port::SeasonFull>,
            SeasonRepositoryError,
        > {
            Ok(None)
        }
    }

    fn make_cmd() -> FinalizeCompetitionCommand {
        FinalizeCompetitionCommand {
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            space_id: SpaceId::new(),
            finalized_by: CoachId::new(),
        }
    }

    #[tokio::test]
    async fn success_emits_competition_ready_event() {
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let result = execute(make_cmd(), &FakeRepo { fail: false }, &bus).await;

        assert!(result.is_ok());
        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.event_type, "CompetitionReady");
    }

    #[tokio::test]
    async fn not_found_returns_error() {
        let result = execute(make_cmd(), &FakeRepo { fail: true }, &new_bus()).await;
        assert!(matches!(
            result,
            Err(FinalizeCompetitionError::SeasonNotFound)
        ));
    }
}
