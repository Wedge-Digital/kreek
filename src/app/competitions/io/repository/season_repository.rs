use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::competition_season::CompetitionSeason;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonBaseInfo, SeasonFull, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use async_trait::async_trait;
use sqlx::PgPool;

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
            .bind(season.name.as_ref())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn find_latest_season_id(
        &self,
        competition_id: &CompetitionId,
    ) -> Result<Option<SeasonId>, SeasonRepositoryError> {
        let id: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/find_latest_season_id.sql"))
                .bind(competition_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        Ok(id.and_then(|s| SeasonId::try_new(&s).ok()))
    }

    async fn find_space_id(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<String>, SeasonRepositoryError> {
        // Le saut, en une jointure : la saison hérite de l'espace de sa
        // compétition et n'en porte pas.
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT c.space_id
               FROM competition_seasons s
               JOIN competitions c ON c.id = s.competition_id
              WHERE s.id = $1",
        )
        .bind(season_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn find_base_info(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            name: String,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/find_base_info.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        Ok(row.map(|r| SeasonBaseInfo { name: r.name }))
    }

    async fn find_rules(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionRules>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rules: Option<String>,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_rules.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.rules) else {
            return Ok(None);
        };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_rules(
        &self,
        season_id: &SeasonId,
        name: &str,
        rules: &CompetitionRules,
    ) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(rules)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_rules.sql"))
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

    async fn find_structure(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionStructure>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            structure: Option<String>,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_structure.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.structure) else {
            return Ok(None);
        };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_structure(
        &self,
        season_id: &SeasonId,
        structure: &CompetitionStructure,
    ) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(structure)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_structure.sql"))
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

    async fn find_invitations(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            invitations: Option<String>,
        }

        let row: Option<Row> =
            sqlx::query_as::<_, Row>(include_str!("sql/seasons/select_invitations.sql"))
                .bind(season_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        let Some(Some(json)) = row.map(|r| r.invitations) else {
            return Ok(None);
        };

        serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))
    }

    async fn save_invitations(
        &self,
        season_id: &SeasonId,
        invitations: &CompetitionInvitations,
    ) -> Result<(), SeasonRepositoryError> {
        let json = serde_json::to_string(invitations)
            .map_err(|e| SeasonRepositoryError::Database(e.to_string()))?;

        let found: Option<String> =
            sqlx::query_scalar(include_str!("sql/seasons/update_invitations.sql"))
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

    async fn find_full(
        &self,
        season_id: &SeasonId,
    ) -> Result<Option<SeasonFull>, SeasonRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            season_id: String,
            season_name: String,
            status: String,
            rules: Option<String>,
            structure: Option<String>,
            competition_id: String,
            competition_name: String,
        }

        let row: Option<Row> = sqlx::query_as(include_str!("sql/seasons/find_season_full.sql"))
            .bind(season_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;

        let Some(r) = row else {
            tracing::warn!(
                "find_full: season {} not found in competition_seasons",
                season_id
            );
            return Ok(None);
        };

        let rules = r
            .rules
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());
        let structure = r
            .structure
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        Ok(Some(SeasonFull {
            season_id: r.season_id,
            season_name: r.season_name,
            status: r.status,
            competition_id: r.competition_id,
            competition_name: r.competition_name,
            rules,
            structure,
        }))
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
