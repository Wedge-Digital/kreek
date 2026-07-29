use async_trait::async_trait;

pub struct GroupWithTeams {
    pub group_id: String,
    pub group_name: String,
    pub position: i32,
    pub team_ids: Vec<String>,
}

#[derive(Debug)]
pub enum GroupRepositoryError {
    Database(String),
}

impl std::fmt::Display for GroupRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupRepositoryError::Database(e) => write!(f, "database error: {}", e),
        }
    }
}

#[async_trait]
pub trait IGroupRepository: Send + Sync {
    async fn find_groups(
        &self,
        season_id: &str,
    ) -> Result<Vec<GroupWithTeams>, GroupRepositoryError>;

    async fn save_assignments(
        &self,
        assignments: &[(String, String)],
    ) -> Result<(), GroupRepositoryError>;

    async fn reset_assignments(&self, season_id: &str) -> Result<(), GroupRepositoryError>;

    async fn assign_team(&self, group_id: &str, team_id: &str) -> Result<(), GroupRepositoryError>;

    async fn unassign_team(&self, team_id: &str) -> Result<(), GroupRepositoryError>;

    async fn ensure_groups_from_structure(
        &self,
        season_id: &str,
        groups: &[(String, String)],
    ) -> Result<(), GroupRepositoryError>;
}
