use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;

#[derive(Debug)]
pub struct SaveCompetitionStructureCommand {
    pub season_id: SeasonId,
    pub structure: CompetitionStructure,
}

#[derive(Debug)]
pub enum SaveCompetitionStructureError {
    SeasonNotFound,
    Database(String),
}

impl From<SeasonRepositoryError> for SaveCompetitionStructureError {
    fn from(e: SeasonRepositoryError) -> Self {
        match e {
            SeasonRepositoryError::SeasonNotFound => Self::SeasonNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: SaveCompetitionStructureCommand,
    repo: &dyn ISeasonRepository,
) -> Result<(), SaveCompetitionStructureError> {
    repo.save_structure(&cmd.season_id, &cmd.structure).await?;
    Ok(())
}
