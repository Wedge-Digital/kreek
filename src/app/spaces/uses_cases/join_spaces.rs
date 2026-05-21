use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::{CoachId, EventId, SpaceId};
use crate::app::spaces::domain::domain_event::SpacesDomainEvent;
use crate::app::spaces::domain::space_repository_port::space_repository_port::{ISpaceRepository, SpaceRepositoryError};
use crate::lib::services::event_bus::event_bus::EventBus;

pub struct JoinSpacesCommand {
    pub coach_id:  CoachId,
    pub space_ids: Vec<SpaceId>,
}

#[derive(Debug)]
pub enum JoinSpacesError {
    Database(String),
}

impl From<SpaceRepositoryError> for JoinSpacesError {
    fn from(e: SpaceRepositoryError) -> Self {
        JoinSpacesError::Database(e.to_string())
    }
}

pub async fn execute(
    cmd: JoinSpacesCommand,
    repo: &dyn ISpaceRepository,
    bus: &EventBus,
) -> Result<(), JoinSpacesError> {
    if cmd.space_ids.is_empty() {
        return Ok(());
    }

    repo.join_spaces(&cmd.space_ids, &cmd.coach_id)
        .await
        .map_err(JoinSpacesError::from)?;

    for space_id in &cmd.space_ids {
        let event = SpacesDomainEvent::UserSubscribedToSpace {
            event_id:     EventId::new(),
            user_id:      cmd.coach_id,
            space_id:     *space_id,
            space_profile: SpaceProfile::SpaceUser,
        };
        let _ = bus.send(event.to_enveloppe());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::app::spaces::domain::space::Space;
    use crate::app::spaces::domain::space_repository_port::space_repository_port::{SpaceSummary, ISpaceRepository};
    use crate::lib::services::event_bus::event_bus::new_bus;

    struct FakeRepo { pub fail: bool }

    #[async_trait]
    impl ISpaceRepository for FakeRepo {
        async fn save(&self, _: &Space) -> Result<(), SpaceRepositoryError> { Ok(()) }
        async fn add_member(&self, _: &SpaceId, _: &CoachId, _: &SpaceProfile) -> Result<(), SpaceRepositoryError> { Ok(()) }
        async fn join_spaces(&self, _: &[SpaceId], _: &CoachId) -> Result<(), SpaceRepositoryError> {
            if self.fail {
                Err(SpaceRepositoryError::Database("db error".into()))
            } else {
                Ok(())
            }
        }
        async fn find_by_id(&self, _: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> { Ok(None) }
        async fn find_by_coach_id(&self, _: &CoachId) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> { Ok(vec![]) }
        async fn find_member_profile(&self, _: &CoachId, _: &SpaceId) -> Result<Option<SpaceProfile>, SpaceRepositoryError> { Ok(None) }
        async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> { Ok(vec![]) }
    }

    fn cmd_with(n: usize) -> JoinSpacesCommand {
        JoinSpacesCommand {
            coach_id:  CoachId::new(),
            space_ids: (0..n).map(|_| SpaceId::new()).collect(),
        }
    }

    #[tokio::test]
    async fn execute_retourne_ok_si_liste_vide() {
        let result = execute(cmd_with(0), &FakeRepo { fail: false }, &new_bus()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_rejoint_les_espaces_et_emet_un_event_par_espace() {
        let bus = new_bus();
        let mut rx = bus.subscribe();
        let cmd = cmd_with(3);

        execute(cmd, &FakeRepo { fail: false }, &bus).await.unwrap();

        let mut count = 0;
        while let Ok(env) = rx.try_recv() {
            assert_eq!(env.event_type.to_string(), "UserRegisteredInSpace");
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn execute_propage_erreur_repository() {
        let result = execute(cmd_with(2), &FakeRepo { fail: true }, &new_bus()).await;
        assert!(matches!(result, Err(JoinSpacesError::Database(_))));
    }
}