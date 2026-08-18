use crate::app::competitions::domain::group_repository_port::IGroupRepository;

#[derive(Debug)]
pub enum ResetError {
    Repository(String),
}

#[tracing::instrument(skip_all, fields(season_id = ?season_id))]
pub async fn execute(season_id: &str, group_repo: &dyn IGroupRepository) -> Result<(), ResetError> {
    group_repo
        .reset_assignments(season_id)
        .await
        .map_err(|e| ResetError::Repository(e.to_string()))?;
    Ok(())
}
