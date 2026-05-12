use crate::app::shared_kernel::authorization::SpaceAuthorization;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::space_name::SpaceName;
use crate::app::spaces::domain::Space::Space;
use crate::app::spaces::domain::ports::{ISpaceRepository, SpaceRepositoryError};

pub struct RegisterNewSpaceCommand {
    pub coach_id:   CoachId,
    pub space_name: SpaceName,
    pub space_logo: CloudinaryImage,
}

#[derive(Debug)]
pub enum RegisterSpaceError {
    SpaceNameAlreadyTaken,
    Database(String),
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
) -> Result<(), RegisterSpaceError> {
    let space = Space::new(SpaceId::new(), cmd.space_name, cmd.space_logo, vec![]);

    repo.save(&space)
        .await
        .map_err(RegisterSpaceError::from)?;

    repo.add_member(&space.id, &cmd.coach_id, &SpaceAuthorization::SpaceAdmin)
        .await
        .map_err(RegisterSpaceError::from)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::spaces::domain::ports::{SpaceRepositoryError, SpaceSummary};
    use crate::app::spaces::domain::Space::Space;
    use async_trait::async_trait;

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
        let result = execute(make_cmd(), &SpaceRepoOk).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_renvoie_space_name_already_taken() {
        let result = execute(make_cmd(), &SpaceRepoNameTaken).await;
        assert!(matches!(result, Err(RegisterSpaceError::SpaceNameAlreadyTaken)));
    }
}