use crate::app::competitions::domain::group_repository_port::{
    GroupRepositoryError, GroupWithTeams, IGroupRepository,
};
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: sqlx::Error) -> GroupRepositoryError {
    GroupRepositoryError::Database(e.to_string())
}

pub struct GroupRepository {
    pool: PgPool,
}

impl GroupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IGroupRepository for GroupRepository {
    async fn find_groups(
        &self,
        season_id: &str,
    ) -> Result<Vec<GroupWithTeams>, GroupRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct GroupRow {
            id: String,
            name: String,
            position: i32,
        }

        let groups = sqlx::query_as::<_, GroupRow>(
            "SELECT id, name, position FROM competition_groups
             WHERE season_id = $1 ORDER BY position, name",
        )
        .bind(season_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let mut result = Vec::with_capacity(groups.len());
        for g in groups {
            #[derive(sqlx::FromRow)]
            struct TeamRow {
                team_id: String,
            }

            let teams = sqlx::query_as::<_, TeamRow>(
                "SELECT team_id FROM competition_group_teams WHERE group_id = $1",
            )
            .bind(&g.id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

            result.push(GroupWithTeams {
                group_id: g.id,
                group_name: g.name,
                position: g.position,
                team_ids: teams.into_iter().map(|t| t.team_id).collect(),
            });
        }

        Ok(result)
    }

    async fn save_assignments(
        &self,
        assignments: &[(String, String)],
    ) -> Result<(), GroupRepositoryError> {
        for (group_id, team_id) in assignments {
            sqlx::query(
                "INSERT INTO competition_group_teams (group_id, team_id)
                 VALUES ($1, $2)
                 ON CONFLICT (group_id, team_id) DO NOTHING",
            )
            .bind(group_id)
            .bind(team_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        }
        Ok(())
    }

    async fn reset_assignments(
        &self,
        season_id: &str,
    ) -> Result<(), GroupRepositoryError> {
        sqlx::query(
            "DELETE FROM competition_group_teams
             WHERE group_id IN (SELECT id FROM competition_groups WHERE season_id = $1)",
        )
        .bind(season_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn assign_team(
        &self,
        group_id: &str,
        team_id: &str,
    ) -> Result<(), GroupRepositoryError> {
        sqlx::query(
            "DELETE FROM competition_group_teams WHERE team_id = $1",
        )
        .bind(team_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        sqlx::query(
            "INSERT INTO competition_group_teams (group_id, team_id) VALUES ($1, $2)",
        )
        .bind(group_id)
        .bind(team_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    async fn unassign_team(
        &self,
        team_id: &str,
    ) -> Result<(), GroupRepositoryError> {
        sqlx::query("DELETE FROM competition_group_teams WHERE team_id = $1")
            .bind(team_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn ensure_groups_from_structure(
        &self,
        season_id: &str,
        groups: &[(String, String)],
    ) -> Result<(), GroupRepositoryError> {
        for (i, (id, name)) in groups.iter().enumerate() {
            sqlx::query(
                "INSERT INTO competition_groups (id, season_id, name, position)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, position = EXCLUDED.position",
            )
            .bind(id)
            .bind(season_id)
            .bind(name)
            .bind(i as i32)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        }
        Ok(())
    }
}
