use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::season_repository_port::{ISeasonRepository, SeasonRepositoryError};
use crate::app::shared_kernel::common_types::SeasonId;

pub struct SaveCompetitionInvitationsCommand {
    pub season_id:   SeasonId,
    pub invitations: CompetitionInvitations,
}

#[derive(Debug)]
pub enum SaveCompetitionInvitationsError {
    SeasonNotFound,
    Database(String),
}

impl From<SeasonRepositoryError> for SaveCompetitionInvitationsError {
    fn from(e: SeasonRepositoryError) -> Self {
        match e {
            SeasonRepositoryError::SeasonNotFound => Self::SeasonNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

pub async fn execute(
    cmd:  SaveCompetitionInvitationsCommand,
    repo: &dyn ISeasonRepository,
) -> Result<(), SaveCompetitionInvitationsError> {
    repo.save_invitations(&cmd.season_id, &cmd.invitations).await?;
    Ok(())
}