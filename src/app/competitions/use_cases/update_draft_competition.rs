use crate::app::competitions::domain::competition::admins_after_edit;
use crate::app::competitions::domain::competition_repository_port::{
    CompetitionRepositoryError, ICompetitionRepository,
};
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};

#[derive(Debug)]
pub struct UpdateDraftCompetitionCommand {
    pub competition_id: CompetitionId,
    pub space_id: SpaceId,
    pub name: CompetitionName,
    pub logo: CloudinaryImage,
    pub admin_ids: Vec<CoachId>,
    /// Celui qui modifie : un admin de la compétition reste dans la liste
    /// (carte 573).
    pub editor_id: CoachId,
}

#[derive(Debug)]
pub enum UpdateDraftCompetitionError {
    CompetitionNotFound,
    CompetitionNameAlreadyTaken,
    Database(String),
}

impl From<CompetitionRepositoryError> for UpdateDraftCompetitionError {
    fn from(e: CompetitionRepositoryError) -> Self {
        match e {
            CompetitionRepositoryError::CompetitionNameAlreadyTaken => {
                UpdateDraftCompetitionError::CompetitionNameAlreadyTaken
            }
            CompetitionRepositoryError::CompetitionNotFound => {
                UpdateDraftCompetitionError::CompetitionNotFound
            }
            CompetitionRepositoryError::Database(msg) => UpdateDraftCompetitionError::Database(msg),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UpdateDraftCompetitionCommand,
    repo: &dyn ICompetitionRepository,
) -> Result<(), UpdateDraftCompetitionError> {
    let current = repo
        .find_base_info(&cmd.competition_id)
        .await?
        .ok_or(UpdateDraftCompetitionError::CompetitionNotFound)?;

    if current.name != cmd.name.value()
        && repo.name_exists_in_space(&cmd.name, &cmd.space_id).await?
    {
        return Err(UpdateDraftCompetitionError::CompetitionNameAlreadyTaken);
    }

    let current_admins: Vec<CoachId> = current
        .admin_ids
        .iter()
        .filter_map(|id| CoachId::try_new(id).ok())
        .collect();
    let admin_ids = admins_after_edit(&current_admins, cmd.admin_ids, &cmd.editor_id);

    repo.update_base_info(&cmd.competition_id, &cmd.name, &cmd.logo, &admin_ids)
        .await?;

    Ok(())
}
