use crate::app::shared_kernel::common_types::{CoachId, Entity, EntityId};
use crate::app::team_creation::create_draft_team::create_draft_team;
use crate::app::shared_kernel::id_service::{FakeIdService, IdService};
use crate::app::shared_kernel::team::{BaseTeamInfo, TeamName};
use crate::app::team_creation::domain::team_draft::DraftTeam;

#[test]
pub fn assert_team_creation_without_logo_is_ok() {

    let jsonized_team = r#"
    {
        "name": "Les Bleus",
        "coach_id": "01F8Z3ZQZQZQZQZQZQZQZQZQZQ"
    }
    "#;
    let base: BaseTeamInfo = serde_json::from_str(jsonized_team).unwrap();
    let id_service = FakeIdService::new();
    let creator_id = id_service.generate_id();
    let team = DraftTeam::new(&id_service, creator_id, base);

    let expected_team_str = r#"{
    "entity_id": "01D39ZY06FGSCTVN4T2V9PKHFZ",
    "created_by": "01D39ZY06FGSCTVN4T2V9PKHFZ",
    "base_infos": {
        "name": "Les Bleus",
        "coach_id": "01F8Z3ZQZQZQZQZQZQZQZQZQZQ",
        "logo_url": null
    }
}"#;
    let serialized_team = serde_json::to_string_pretty(&team).unwrap();
    let x_team: DraftTeam = serde_json::from_str(&expected_team_str).unwrap();
    let expected_team = serde_json::to_string_pretty(&x_team).unwrap();
    assert_eq!(serialized_team, expected_team);
}

#[test]
pub fn assert_new_team_is_given_a_new_id() {
    let jsonized_team = r#"
    {
        "name": "Les Bleus",
        "coach_id": "01F8Z3ZQZQZQZQZQZQZQZQZQZQ"
    }
    "#;
    let base: BaseTeamInfo = serde_json::from_str(jsonized_team).unwrap();
    let id_service = FakeIdService::new();
    let creator_id = id_service.generate_id();
    let team = DraftTeam::new(&id_service, creator_id, base);
    let id = team.get_id();
    let expected_id = EntityId::from_string("01D39ZY06FGSCTVN4T2V9PKHFZ").unwrap();
    assert_eq!(id, expected_id);
}

#[test]
pub fn assert_factory_method_is_ok() {
    let id_service = FakeIdService::new();
    let coach_id = CoachId::from_string("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap();
    let creator_id = id_service.generate_id();
    let team = create_draft_team(&id_service,
                                            creator_id,
                                            TeamName::try_new("Les Bleus".to_string()).unwrap(),
                                            coach_id,
                                            None);
    let id = team.get_id();
    let expected_id = EntityId::from_string("01D39ZY06FGSCTVN4T2V9PKHFZ").unwrap();
    let expected_team_str = r#"{
    "entity_id": "01D39ZY06FGSCTVN4T2V9PKHFZ",
    "created_by": "01D39ZY06FGSCTVN4T2V9PKHFZ",
    "base_infos": {
        "name": "Les Bleus",
        "coach_id": "01F8Z3ZQZQZQZQZQZQZQZQZQZQ",
        "logo_url": null
    }
}"#;
    let serialized_team = serde_json::to_string_pretty(&team).unwrap();
    let x_team: DraftTeam = serde_json::from_str(&expected_team_str).unwrap();
    let expected_team = serde_json::to_string_pretty(&x_team).unwrap();
    assert_eq!(serialized_team, expected_team);
    assert_eq!(id, expected_id);
}

#[test]
pub fn assert_deserialize_team_to_hash_map() {
    let id_service = FakeIdService::new();
    let coach_id = CoachId::from_string("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap();
    let creator_id = id_service.generate_id();
    let team = create_draft_team(&id_service,
                                            creator_id,
                                            TeamName::try_new("Les Bleus".to_string()).unwrap(),
                                            coach_id,
                                            None);
    let serialized_team = serde_json::to_string_pretty(&team).unwrap();
    let team_map: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(&serialized_team).unwrap();
    assert_eq!(team_map.get("entity_id").unwrap().as_str().unwrap(), "01D39ZY06FGSCTVN4T2V9PKHFZ");
    assert_eq!(team_map.get("base_infos").unwrap().get("name").unwrap().as_str().unwrap(), "Les Bleus");
}