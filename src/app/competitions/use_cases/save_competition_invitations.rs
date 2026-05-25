use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
use crate::app::shared_kernel::common_types::CompetitionId;

pub struct SaveCompetitionInvitationsCommand {
    pub competition_id: CompetitionId,
    pub invitations:    CompetitionInvitations,
}

#[derive(Debug)]
pub enum SaveCompetitionInvitationsError {
    CompetitionNotFound,
    Database(String),
}

impl From<CompetitionRepositoryError> for SaveCompetitionInvitationsError {
    fn from(e: CompetitionRepositoryError) -> Self {
        match e {
            CompetitionRepositoryError::CompetitionNotFound => Self::CompetitionNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

pub async fn execute(
    cmd:  SaveCompetitionInvitationsCommand,
    repo: &dyn ICompetitionRepository,
) -> Result<(), SaveCompetitionInvitationsError> {
    repo.save_invitations(&cmd.competition_id, &cmd.invitations).await?;
    Ok(())
}