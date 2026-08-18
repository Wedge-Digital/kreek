use crate::app::competitions::domain::group_repository_port::IGroupRepository;

#[derive(Debug)]
pub enum AssignError {
    Repository(String),
}

#[tracing::instrument(skip_all, fields(team_id = ?team_id))]
pub async fn execute(
    team_id: &str,
    group_id: &str,
    group_repo: &dyn IGroupRepository,
) -> Result<(), AssignError> {
    group_repo
        .assign_team(group_id, team_id)
        .await
        .map_err(|e| AssignError::Repository(e.to_string()))?;
    Ok(())
}
