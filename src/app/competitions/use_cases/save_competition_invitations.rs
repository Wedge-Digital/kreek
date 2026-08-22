use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;

#[derive(Debug)]
pub struct SaveCompetitionInvitationsCommand {
    pub season_id: SeasonId,
    pub invitations: CompetitionInvitations,
    /// L'étape 4 du magicien porte désormais les deux : le widget de réglage y
    /// est en mode différé, et c'est ce POST qui persiste ses cases.
    pub notifications: CompetitionNotifications,
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

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: SaveCompetitionInvitationsCommand,
    repo: &dyn ISeasonRepository,
) -> Result<(), SaveCompetitionInvitationsError> {
    repo.save_invitations(&cmd.season_id, &cmd.invitations, &cmd.notifications)
        .await?;
    Ok(())
}
