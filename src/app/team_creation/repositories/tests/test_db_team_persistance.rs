use sqlx::{PgPool};
use sqlx::postgres::PgPoolOptions;
use crate::app::global_types::global_type::{Entity, EntityId};
use crate::app::team_creation::repositories::db_team_persistance::DbTeamPersistance;
use crate::app::team_creation::repositories::tests::draft_team_fixtures::create_draft_team_fixture;
use crate::app::team_creation::repositories::team_persistance::TeamPersistance;

fn get_ok_connection_string() -> String {
    let returned = dotenvy::dotenv();
    if returned.is_err() {
        panic!("Error loading .env file");
    }
    println!("DATABASE_URL: {:?}", dotenvy::var("DATABASE_URL"));
    return dotenvy::var("DATABASE_URL").unwrap();
}

fn get_wrong_connection_string() -> String {
    return "postgresql://root:root@localhost/no_db".to_string();
}

fn get_wrong_table_connection_string() -> String {
    return "postgresql://root:root@localhost:5425/no_db".to_string();
}

async fn connect_db_driver() -> PgPool {
    let conn_result = PgPoolOptions::new()
        .max_connections(5)
        .connect( & get_ok_connection_string())
        .await;
    return conn_result.unwrap();
}

#[tokio::test]
pub async fn assert_db_driver_initialization_should_be_ok() {
    let conn_result = PgPoolOptions::new()
        .max_connections(5)
        .connect(&get_ok_connection_string())
        .await;
    assert_eq!(conn_result.is_err(), false);
}

#[tokio::test]
pub async fn assert_db_driver_timeout() {
    let conn_result = PgPoolOptions::new()
        .max_connections(5)
        .connect(&get_wrong_connection_string())
        .await;
    assert_eq!(conn_result.is_err(), true);
    assert_eq!(conn_result.err().unwrap().to_string(), "pool timed out while waiting for an open connection".to_string());
}

#[tokio::test]
pub async fn assert_db_driver_wrong_table() {
    let conn_result = PgPoolOptions::new()
        .max_connections(5)
        .connect(&get_wrong_table_connection_string())
        .await;
    assert_eq!(conn_result.is_err(), true);
    assert_eq!(conn_result.err().unwrap().to_string(), "error returned from database: database \"no_db\" does not exist".to_string());
}

#[sqlx::test]
pub async fn assert_ping_db_should_be_ok(pool: PgPool) {
    let db_persistance = DbTeamPersistance::new(&pool);
    let ping_result = db_persistance.ping().await;
    assert_eq!(ping_result.is_ok(), true);
}

#[sqlx::test]
pub async fn assert_a_team_shall_be_stored(pool: PgPool) {
    let team_to_store = create_draft_team_fixture();
    let mut db_persistance = DbTeamPersistance::new(&pool);
    let res = db_persistance.save(&team_to_store).await;
    assert_eq!(res.is_ok(), true);
}

#[sqlx::test]
pub async fn assert_a_stored_team_shall_be_retrieved(pool: PgPool) {
    let team_to_store = create_draft_team_fixture();
    let mut db_persistance = DbTeamPersistance::new(&pool);
    let res = db_persistance.save(&team_to_store).await;
    let stored_team = db_persistance.get_by_id(team_to_store.get_id()).await;
    assert_eq!(stored_team.is_some(), true);
    assert_eq!(stored_team.unwrap(), team_to_store);
}

#[sqlx::test(fixtures("draft_team"))]
pub async fn assert_save_a_team_already_existing_should_return_error(pool: PgPool) {
    let team_to_store = create_draft_team_fixture();
    let mut db_persistance = DbTeamPersistance::new(&pool);
    let res2 = db_persistance.save(&team_to_store).await;
    assert_eq!(res2.is_err(), true);
    assert_eq!(res2.err().unwrap(), "error returned from database: duplicate key value violates unique constraint \"team_creation__draft_team_pkey\"".to_string());
}

#[sqlx::test]
pub async fn assert_a_retrieve_a_non_existant_team_shall_return_none(pool: PgPool) {
    let mut db_persistance = DbTeamPersistance::new(&pool);
    let non_existing_id = EntityId::from_string("01D39ZY06FGSCTVN4T2V9PKHFZ").unwrap();
    let stored_team = db_persistance.get_by_id(non_existing_id).await;
    assert_eq!(stored_team.is_some(), false);
}
