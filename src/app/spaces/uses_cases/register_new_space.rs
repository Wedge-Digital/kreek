use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId, UserId};
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::spaces::domain::domain_event::SpacesDomainEvent;
use crate::app::spaces::domain::space::Space;
use crate::app::spaces::domain::space_repository_port::space_repository_port::{
    ISpaceRepository, SpaceRepositoryError,
};
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::{
    ISpaceUserCacheRepository, SpaceUserCacheRepositoryError,
};
use crate::common::services::event_bus::event_bus::EventBus;

pub struct RegisterNewSpaceCommand {
    pub coach_id: CoachId,
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
            SpaceUserCacheRepositoryError::UserNotFoundInCache => RegisterSpaceError::CoachNotFound,
            SpaceUserCacheRepositoryError::UsernameNameAlreadyPresentInCache => {
                RegisterSpaceError::Database("username already in cache".into())
            }
            SpaceUserCacheRepositoryError::Database(msg) => RegisterSpaceError::Database(msg),
        }
    }
}

impl From<SpaceRepositoryError> for RegisterSpaceError {
    fn from(e: SpaceRepositoryError) -> Self {
        match e {
            SpaceRepositoryError::SpaceNameAlreadyTaken => {
                RegisterSpaceError::SpaceNameAlreadyTaken
            }
            SpaceRepositoryError::CoachAlreadyMember => {
                RegisterSpaceError::Database("coach already member on brand new space".into())
            }
            SpaceRepositoryError::Database(msg) => RegisterSpaceError::Database(msg),
        }
    }
}

pub async fn execute(
    cmd: RegisterNewSpaceCommand,
    repo: &dyn ISpaceRepository,
    user_cache: &dyn ISpaceUserCacheRepository,
    bus: &EventBus,
) -> Result<(), RegisterSpaceError> {
    let space = Space::new(SpaceId::new(), cmd.space_name, cmd.space_logo, vec![]);

    let _curent_user = user_cache.find_user_by_id(&cmd.coach_id).await?;

    repo.save(&space).await.map_err(RegisterSpaceError::from)?;

    repo.add_member(&space.id, &cmd.coach_id, &SpaceProfile::SpaceAdmin)
        .await
        .map_err(RegisterSpaceError::from)?;

    let space_created_payload = SpacesDomainEvent::SpaceCreated {
        event_id: UserId::new(),
        created_by: cmd.coach_id.clone(),
        space_id: space.id.clone(),
        space_name: space.name.clone(),
        space_logo: space.logo.clone(),
    };

    let _ = bus.send(space_created_payload.to_enveloppe());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::spaces::domain::space::Space;
    use crate::app::spaces::domain::space_repository_port::space_repository_port::{
        ISpaceRepository, SpaceRepositoryError, SpaceSummary,
    };
    use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::{
        ISpaceUserCacheRepository, SpaceUserCacheRepositoryError,
    };
    use crate::app::spaces::domain::user::User as SpaceUser;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;

    struct FakeUserCache {
        pub user: Option<SpaceUser>,
    }
    #[async_trait]
    impl ISpaceUserCacheRepository for FakeUserCache {
        async fn add_user(&self, _: &SpaceUser) -> Result<(), SpaceUserCacheRepositoryError> {
            Ok(())
        }
        async fn find_user_by_id(
            &self,
            _: &CoachId,
        ) -> Result<SpaceUser, SpaceUserCacheRepositoryError> {
            self.user
                .clone()
                .ok_or(SpaceUserCacheRepositoryError::UserNotFoundInCache)
        }
        async fn find_all_users(&self) -> Result<Vec<SpaceUser>, SpaceUserCacheRepositoryError> {
            Ok(vec![])
        }
        async fn list_members_for_space(
            &self,
            _: &SpaceId,
        ) -> Result<Vec<SpaceUser>, SpaceUserCacheRepositoryError> {
            Ok(vec![])
        }
    }

    fn fake_user(coach_id: &CoachId) -> SpaceUser {
        use crate::app::shared_kernel::identity::coach_name::CoachName;
        use crate::app::shared_kernel::identity::email::Email;
        use crate::app::shared_kernel::identity::ids::CloudinaryImage;
        SpaceUser {
            id: coach_id.clone(),
            name: CoachName::try_new("Coach").unwrap(),
            icon: Some(
                CloudinaryImage::try_new("https://res.cloudinary.com/demo/image/upload/sample.jpg")
                    .unwrap(),
            ),
            email: Email::try_new("coach@example.com").unwrap(),
        }
    }

    struct SpaceRepoOk;

    #[async_trait]
    impl ISpaceRepository for SpaceRepoOk {
        async fn save(&self, _space: &Space) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }
        async fn add_member(
            &self,
            _space_id: &SpaceId,
            _coach_id: &CoachId,
            _profile: &SpaceProfile,
        ) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }

        async fn join_spaces(
            &self,
            _space_ids: &[SpaceId],
            _coach_id: &CoachId,
        ) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }

        async fn find_by_id(&self, _id: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> {
            Ok(None)
        }

        async fn find_by_coach_id(
            &self,
            _coach_id: &CoachId,
        ) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }

        async fn find_member_profile(
            &self,
            _coach_id: &CoachId,
            _space_id: &SpaceId,
        ) -> Result<Option<SpaceProfile>, SpaceRepositoryError> {
            Ok(Some(SpaceProfile::SpaceUser))
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
        async fn add_member(
            &self,
            _space_id: &SpaceId,
            _coach_id: &CoachId,
            _profile: &SpaceProfile,
        ) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }

        async fn join_spaces(
            &self,
            _space_ids: &[SpaceId],
            _coach_id: &CoachId,
        ) -> Result<(), SpaceRepositoryError> {
            Ok(())
        }

        async fn find_by_id(&self, _id: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> {
            Ok(None)
        }

        async fn find_by_coach_id(
            &self,
            _coach_id: &CoachId,
        ) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }

        async fn find_member_profile(
            &self,
            _coach_id: &CoachId,
            _space_id: &SpaceId,
        ) -> Result<Option<SpaceProfile>, SpaceRepositoryError> {
            Ok(Some(SpaceProfile::SpaceUser))
        }

        async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
            Ok(vec![])
        }
    }

    fn make_cmd() -> RegisterNewSpaceCommand {
        RegisterNewSpaceCommand {
            coach_id: CoachId::new(),
            space_name: SpaceName::try_new("LigueAlpha").unwrap(),
            space_logo: CloudinaryImage::try_new(
                "https://res.cloudinary.com/demo/image/upload/sample.jpg",
            )
            .unwrap(),
        }
    }

    #[tokio::test]
    async fn execute_cree_espace_et_ajoute_fondateur_admin() {
        let cmd = make_cmd();
        let user = fake_user(&cmd.coach_id);
        let cache = FakeUserCache { user: Some(user) };
        let result = execute(cmd, &SpaceRepoOk, &cache, &new_bus()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_renvoie_space_name_already_taken() {
        let cmd = make_cmd();
        let user = fake_user(&cmd.coach_id);
        let cache = FakeUserCache { user: Some(user) };
        let result = execute(cmd, &SpaceRepoNameTaken, &cache, &new_bus()).await;
        assert!(matches!(
            result,
            Err(RegisterSpaceError::SpaceNameAlreadyTaken)
        ));
    }
}
