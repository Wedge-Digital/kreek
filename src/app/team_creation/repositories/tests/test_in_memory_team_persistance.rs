use crate::app::global_types::global_type::Entity;
use crate::app::team_creation::repositories::in_memory_team_persistance::InMemoryTeamPersistance;
use crate::app::team_creation::repositories::team_persistance:: TeamPersistance;
use crate::app::team_creation::repositories::tests::draft_team_fixtures::{create_draft_team_fixture, create_draft_team_fixture_with_logo};

#[tokio::test]
pub async fn assert_team_memory_persistance_should_store_a_team() {

    let team_to_store = create_draft_team_fixture();
    let mut persistance = InMemoryTeamPersistance::new();
    assert_eq!(persistance.get_all().await.len(), 0);
    let res = persistance.save(&team_to_store);
    assert_eq!(res.await.is_ok(), true);
    assert_eq!(persistance.get_all().await.len(), 1);
}

#[tokio::test]
pub async fn assert_team_memory_persistance_should_be_able_to_retrieve_a_team() {
    let team_to_store = create_draft_team_fixture_with_logo();
    let mut persistance = InMemoryTeamPersistance::new();
    let res = persistance.save(&team_to_store);
    assert_eq!(res.await.is_ok(), true);
    let stored_team = persistance.get_by_id(team_to_store.get_id()).await;
    assert_eq!(stored_team.is_some(), true);
    assert_eq!(stored_team.unwrap(), team_to_store);
}
