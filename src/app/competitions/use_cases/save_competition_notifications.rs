use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;

#[derive(Debug)]
pub struct SaveCompetitionNotificationsCommand {
    pub season_id: SeasonId,
    pub notifications: CompetitionNotifications,
}

#[derive(Debug)]
pub enum SaveCompetitionNotificationsError {
    SeasonNotFound,
    Database(String),
}

impl From<SeasonRepositoryError> for SaveCompetitionNotificationsError {
    fn from(e: SeasonRepositoryError) -> Self {
        match e {
            SeasonRepositoryError::SeasonNotFound => Self::SeasonNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

/// Écrit les quatre réglages tels qu'ils arrivent.
///
/// Aucun filtrage sur l'applicabilité, et c'est R6 : une notification cochée
/// puis rendue inapplicable — parce que le calendrier a été retiré, par
/// exemple — **reste cochée**, et redevient active si le calendrier revient.
/// Refuser l'écriture ici perdrait silencieusement l'intention de
/// l'organisateur.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: SaveCompetitionNotificationsCommand,
    repo: &dyn ISeasonRepository,
) -> Result<(), SaveCompetitionNotificationsError> {
    repo.save_notifications(&cmd.season_id, &cmd.notifications)
        .await?;
    Ok(())
}
