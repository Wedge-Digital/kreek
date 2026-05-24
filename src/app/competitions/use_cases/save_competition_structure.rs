use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::shared_kernel::common_types::CompetitionId;

pub struct SaveCompetitionStructureCommand {
    pub competition_id: CompetitionId,
    pub structure:      CompetitionStructure,
}

#[derive(Debug)]
pub enum SaveCompetitionStructureError {
    CompetitionNotFound,
    Database(String),
}

impl From<CompetitionRepositoryError> for SaveCompetitionStructureError {
    fn from(e: CompetitionRepositoryError) -> Self {
        match e {
            CompetitionRepositoryError::CompetitionNotFound => Self::CompetitionNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

pub async fn execute(
    cmd:  SaveCompetitionStructureCommand,
    repo: &dyn ICompetitionRepository,
) -> Result<(), SaveCompetitionStructureError> {
    repo.save_structure(&cmd.competition_id, &cmd.structure).await?;
    Ok(())
}
