use crate::app::competitions::domain::group_repository_port::IGroupRepository;

#[derive(Debug)]
pub enum AssignError {
    Repository(String),
}

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
