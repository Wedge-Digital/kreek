use time::OffsetDateTime;
use crate::app::shared_kernel::authorization::SpaceAuthorization;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId, UserId};
use crate::app::shared_kernel::space_name::SpaceName;
use crate::app::spaces::domain::domain_event::{SpacesDomainEvent, SPACE_CREATED, USER_PROMOTED_TO_SPACE_ADMIN, USER_SUBSCRIBED_TO_SPACE};
use crate::app::spaces::domain::space::Space;
use crate::app::spaces::domain::space_repository_port::space_repository_port::{ISpaceRepository, SpaceRepositoryError};
use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::{ISpaceUserCacheRepository, SpaceUserCacheRepositoryError};
use crate::lib::domain_event::DomainEventEnvelope;
use crate::lib::services::event_bus::event_bus::IEventPublisher;

pub struct RegisterNewSpaceCommand {
    pub coach_id:   CoachId,
    pub space_name: SpaceName,
    pub space_logo: CloudinaryImage,
}

#[derive(Debug)]
pub enum RegisterSpaceError {
    SpaceNameAlreadyTaken,
    CoachNotFound,
    Database(String),
}

impl From<SpaceUserCacheRepositoryError> for RegisterSpaceError {
    fn from(e: SpaceUserCacheRepositoryError) -> Self {
        match e {
            SpaceUserCacheRepositoryError::UsernameNameAlreadyPresentInCache => RegisterSpaceError::Database("user not found in cache".into()),
            SpaceUserCacheRepositoryError::Database(msg)                     => RegisterSpaceError::Database(msg),
        }
    }
}

impl From<SpaceRepositoryError> for RegisterSpaceError {
    fn from(e: SpaceRepositoryError) -> Self {
        match e {
            SpaceRepositoryError::SpaceNameAlreadyTaken => RegisterSpaceError::SpaceNameAlreadyTaken,
            SpaceRepositoryError::CoachAlreadyMember    => RegisterSpaceError::Database("coach already member on brand new space".into()),
            SpaceRepositoryError::Database(msg)         => RegisterSpaceError::Database(msg),
        }
    }
}

pub async fn execute(
    cmd: RegisterNewSpaceCommand,
    repo: &dyn ISpaceRepository,
    user_cache: &dyn ISpaceUserCacheRepository,
    bus: &dyn IEventPublisher,
) -> Result<(), RegisterSpaceError> {
    let space = Space::new(SpaceId::new(), cmd.space_name, cmd.space_logo, vec![]);

    let user_search = user_cache.find_by_id(&cmd.coach_id).await?;

    if user_search.is_none() {
        return Err(RegisterSpaceError::CoachNotFound);
    }
    let curent_user = user_search.unwrap();

    repo.save(&space)
        .await
        .map_err(RegisterSpaceError::from)?;

    repo.add_member(&space.id, &cmd.coach_id, &SpaceAuthorization::SpaceAdmin)
        .await
        .map_err(RegisterSpaceError::from)?;

    let space_id = space.id.to_string();
    let user_id = cmd.coach_id.to_string();
    let space_name = space.name.to_string();
    let space_logo = space.logo.to_string();

    let space_created_payload = SpacesDomainEvent::SpaceCreated {
        event_id:   UserId::new().to_string(),
        created_by: user_id.clone(),
        space_id:   space_id.clone(),
        space_name: space_name.clone(),
        space_logo: space_logo.clone(),
    };

    bus.publish(DomainEventEnvelope {
        event_id:   UserId::new().to_string(),
        emitter:    user_id.clone(),
        event_type: SPACE_CREATED.into(),
        tags:       serde_json::json!({ "user_id": user_id }),
        payload:    serde_json::json!(space_created_payload),
        occurred_at: OffsetDateTime::now_utc(),
    });

    let user_subscribed_payload = SpacesDomainEvent::UserSubscribedToSpace {
        event_id:   UserId::new().to_string(),
        user_id:    user_id.clone(),
        user_name:  curent_user.name.clone(),
        email:      curent_user.email.clone(),
        space_id:   space_id.clone(),
    };

    bus.publish(DomainEventEnvelope {
        event_id:   UserId::new().to_string(),
        emitter:    user_id.clone(),
        event_type: USER_SUBSCRIBED_TO_SPACE.into(),
        tags:       serde_json::json!({ "user_id": user_id }),
        payload:    serde_json::json!(user_subscribed_payload),
        occurred_at: OffsetDateTime::now_utc(),
    });

    let user_promoted_payload = SpacesDomainEvent::UserPromotedToSpaceAdmin {
        event_id:   UserId::new().to_string(),
        user_id:    user_id.clone(),
    };

    bus.publish(DomainEventEnvelope {
        event_id:   UserId::new().to_string(),
        emitter:    user_id.clone(),
        event_type: USER_PROMOTED_TO_SPACE_ADMIN.into(),
        tags:       serde_json::json!({ "user_id": user_id }),
        payload:    serde_json::json!(user_promoted_payload),
        occurred_at: OffsetDateTime::now_utc(),
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::spaces::domain::space::Space;
    use async_trait::async_trait;
    use crate::app::spaces::domain::space_repository_port::space_repository_port::{ISpaceRepository, SpaceRepositoryError, SpaceSummary};
    use crate::app::spaces::domain::space_repository_port::user_cache_repository_ports::{ISpaceUserCacheRepository, SpaceUserCacheRepositoryError};
    use crate::app::spaces::domain::user::User as SpaceUser;
    use crate::lib::domain_event::DomainEventEnvelope;
    use crate::lib::services::event_bus::event_bus::IEventPublisher;

    struct FakeUserCache { pub user: Option<SpaceUser> }
    #[async_trait]
    impl ISpaceUserCacheRepository for FakeUserCache {
        async fn save(&self, _: &SpaceUser) -> Result<(), SpaceUserCacheRepositoryError> { Ok(()) }
        async fn find_by_id(&self, _: &CoachId) -> Result<Option<SpaceUser>, SpaceUserCacheRepositoryError> {
            Ok(self.user.clone())
        }
        async fn find_all(&self) -> Result<Vec<SpaceUser>, SpaceUserCacheRepositoryError> { Ok(vec![]) }
    }

    struct FakeBus;
    impl IEventPublisher for FakeBus {
        fn publish(&self, _: DomainEventEnvelope) {}
    }

    fn fake_user(coach_id: &CoachId) -> SpaceUser {
        use crate::app::shared_kernel::coach_name::CoachName;
        use crate::app::shared_kernel::common_types::CloudinaryImage;
        use crate::app::shared_kernel::email::Email;
        SpaceUser {
            id:    coach_id.clone(),
            name:  CoachName::try_new("Coach").unwrap(),
            icon:  CloudinaryImage::try_new("https://res.cloudinary.com/demo/image/upload/sample.jpg").unwrap(),
            email: Email::try_new("coach@example.com").unwrap(),
        }
    }

    struct SpaceRepoOk;

    #[async_trait]
    impl ISpaceRepository for SpaceRepoOk {
        async fn save(&self, _space: &Space) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }
        async fn add_member(&self, _space_id: &SpaceId, _coach_id: &CoachId, _profile: &SpaceAuthorization) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }
        async fn find_by_id(&self, _id: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> {
            Ok(None)
        }

        async fn find_by_coach_id(&self, coach_id: &CoachId) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }

        async fn find_member_profile(&self, coach_id: &CoachId, space_id: &SpaceId) -> Result<Option<SpaceAuthorization>, SpaceRepositoryError> {
            Ok(Some(SpaceAuthorization::SimpleUser))
        }

        async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }
    }

    struct SpaceRepoNameTaken;

    #[async_trait]
    impl ISpaceRepository for SpaceRepoNameTaken {
        async fn save(&self, _space: &Space) -> Result<(), SpaceRepositoryError> {
            Err(SpaceRepositoryError::SpaceNameAlreadyTaken)
        }
        async fn add_member(&self, _space_id: &SpaceId, _coach_id: &CoachId, _profile: &SpaceAuthorization) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }
        async fn find_by_id(&self, _id: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> {
            Ok(None)
        }

        async fn find_by_coach_id(&self, coach_id: &CoachId) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }

        async fn find_member_profile(&self, coach_id: &CoachId, space_id: &SpaceId) -> Result<Option<SpaceAuthorization>, SpaceRepositoryError> {
            Ok(Some(SpaceAuthorization::SimpleUser))
        }

        async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }
    }

    fn make_cmd() -> RegisterNewSpaceCommand {
        RegisterNewSpaceCommand {
            coach_id:   CoachId::new(),
            space_name: SpaceName::try_new("LigueAlpha").unwrap(),
            space_logo: CloudinaryImage::try_new(
                "https://res.cloudinary.com/demo/image/upload/sample.jpg",
            ).unwrap(),
        }
    }

    #[tokio::test]
    async fn execute_cree_espace_et_ajoute_fondateur_admin() {
        let cmd    = make_cmd();
        let user   = fake_user(&cmd.coach_id);
        let cache  = FakeUserCache { user: Some(user) };
        let result = execute(cmd, &SpaceRepoOk, &cache, &FakeBus).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_renvoie_space_name_already_taken() {
        let cmd   = make_cmd();
        let user  = fake_user(&cmd.coach_id);
        let cache = FakeUserCache { user: Some(user) };
        let result = execute(cmd, &SpaceRepoNameTaken, &cache, &FakeBus).await;
        assert!(matches!(result, Err(RegisterSpaceError::SpaceNameAlreadyTaken)));
    }
}