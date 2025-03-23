use sqlx::Executor;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Json;
use sqlx::types::time::OffsetDateTime;
use crate::app::global_types::global_type::{Entity, EntityId};
use crate::app::team_creation::repositories::team_persistance::TeamPersistance;
use crate::app::team_creation::team_draft::DraftTeam;

pub struct DbTeamPersistance {
    database: sqlx::PgPool
}

struct DraftTeamRow {
    entity_id: EntityId,
    created_by: EntityId,
    coach_id: EntityId,
    serialized: Json<DraftTeam>,
    created_at: String,
    updated_at: String,
}

impl DbTeamPersistance {
    pub fn new(db: &sqlx::PgPool) -> Self {
        return DbTeamPersistance{database: db.clone()};
    }

    pub async fn init(connection_string: &String) -> Result<sqlx::PgPool, sqlx::Error> {
        return PgPoolOptions::new()
            .max_connections(5)
            .connect(connection_string)
            .await;
    }

    pub async fn ping(&self) -> Result<(), String> {
        let request = r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public'
            AND table_name = 'nom_de_la_table'
        );"#;
        let query_result = self.database.execute(sqlx::query(request)).await; // prepared, cached query
        if query_result.is_err() {
            return Err("Error while pinging the database".to_string());
        }
        return Ok(());
    }
}

impl TeamPersistance for DbTeamPersistance {

    async fn save(&mut self, team: &DraftTeam) -> Result<(), String> {
        let serialized = serde_json::to_value(team).unwrap();
        let now = OffsetDateTime::now_utc();
        let strid = team.get_id().to_string();
        let request = sqlx::query_file!("src/app/team_creation/repositories/sql/insert_draft_team.sql",
            team.get_id().to_string(),
            team.get_created_by().to_string(),
            "01D39ZY06FGSCTVN4T2V9PKHFZ",
            serialized,
            now,
            now);
        let query_result = request.execute(&self.database).await;
        println!("Query result: {:?}", query_result);
        if query_result.is_err() {
            return Err(query_result.err().unwrap().to_string());
        }
        return Ok(());
    }

    async fn update(&mut self, team: DraftTeam) -> Result<(), String> {
        return Ok(());
    }

    async fn add_or_update(&mut self, team: DraftTeam) -> Result<(), String> {
        return Ok(());
    }

    async fn get_by_id(&self, id: EntityId) -> Option<DraftTeam> {
        let request = sqlx::query_file!("src/app/team_creation/repositories/sql/get_by_id.sql", id.to_string());
        let query_result = request.fetch_optional(&self.database).await;
        println!("Query result: {:?}", query_result);
        if query_result.is_err() {
            return None;
        }
        let found_record = query_result.unwrap();
        if found_record.is_some() {
            return serde_json::from_value(found_record.unwrap().serialized).unwrap();
        }
        return None;
    }

    async fn delete(&mut self, team: DraftTeam) -> Result<(), String> {
        return Ok(());
    }

    async fn get_all(&self) -> Vec<DraftTeam> {
        todo!()
    }
}