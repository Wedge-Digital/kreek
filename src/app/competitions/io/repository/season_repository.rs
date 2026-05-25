use async_trait::async_trait;
use sqlx::PgPool;
use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_season::CompetitionSeason;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::competitions::domain::season_repository_port::{ISeasonRepository, SeasonBaseInfo, SeasonRepositoryError};
use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};

fn db_err(e: impl std::fmt::Display) -> SeasonRepositoryError {
    SeasonRepositoryError::Database(e.to_string())
}

#[derive(Clone)]
pub struct SeasonRepository {
    pool: PgPool,
}

impl SeasonRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ISeasonRepository for SeasonRepository {
    async fn save(&self, season: &CompetitionSeason) -> Result<(), SeasonRepositoryError> {
        sqlx::query(include_str!("sql/seasons/insert_season.sql"))
            .bind(season.id.to_string())
            .bind(season.competition_id.to_string())
            .bind(&season.name)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn find_latest_season_id(&self, competition_id: &CompetitionId) -> Result<Option<SeasonId>, SeasonRepositoryError> {
        let id: Option<String> = sqlx::query_scalar(include_str!("sql/seasons/find_latest_season_id.sql"))
            .bind(competition_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(id.and_then(|s| SeasonId::try_new(&s).ok()))
    }

    async fn find_base_info(&self, season_id: &SeasonId) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row { name: String }

        let row: Option<Row> = sqlx::query_as::<_, Row>(include_str!("sql/seasons/find_base_info.sql"))
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        Ok(row.map(|r| SeasonBaseInfo { name: r.name }))
    }

    async fn find_rules(&self, season_id: &SeasonId) -> Result<Option<CompetitionRules>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row { rules: Option<String> }

        let row: Option<Row> = sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_rules.sql"))
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.rules) else { return Ok(None) };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_rules(&self, season_id: &SeasonId, name: &str, rules: &CompetitionRules) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(rules)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> = sqlx::query_scalar(include_str!("sql/seasons/update_rules.sql"))
            .bind(name)
            .bind(json)
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }

    async fn find_structure(&self, season_id: &SeasonId) -> Result<Option<CompetitionStructure>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row { structure: Option<String> }

        let row: Option<Row> = sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_structure.sql"))
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.structure) else { return Ok(None) };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_structure(&self, season_id: &SeasonId, structure: &CompetitionStructure) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(structure)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> = sqlx::query_scalar(include_str!("sql/seasons/update_structure.sql"))
            .bind(json)
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }

    async fn find_invitations(&self, season_id: &SeasonId) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row { invitations: Option<String> }

        let row: Option<Row> = sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_invitations.sql"))
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.invitations) else { return Ok(None) };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_invitations(&self, season_id: &SeasonId, invitations: &CompetitionInvitations) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(invitations)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> = sqlx::query_scalar(include_str!("sql/seasons/update_invitations.sql"))
            .bind(json)
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }

    async fn set_ready(&self, season_id: &SeasonId) -> Result<(), SeasonRepositoryError> {
        let found: Option<String> = sqlx::query_scalar(include_str!("sql/seasons/set_ready.sql"))
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        if found.is_none() {
            return Err(SeasonRepositoryError::SeasonNotFound);
        }
        Ok(())
    }
}