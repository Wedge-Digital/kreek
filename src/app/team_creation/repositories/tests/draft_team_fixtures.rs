use crate::app::team_creation::common_types::{CoachId, UserId};
use crate::app::team_creation::create_draft_team::create_draft_team;
use crate::app::team_creation::team_draft::DraftTeam;
use crate::services::id_service::FakeIdService;

pub fn create_draft_team_fixture() -> DraftTeam {
    let team_to_store = create_draft_team(
        &FakeIdService::new(),
        UserId::from_string("01JQ1F41YST89JHM4VY7190VJA").unwrap(),
        "Les Bleus".to_string(),
        CoachId::from_string("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap(),
        None
    );
    return team_to_store;
}

pub fn create_draft_team_fixture_with_logo() -> DraftTeam {
    let team_to_store = create_draft_team(
        &FakeIdService::new(),
        UserId::from_string("01JQ1F57G654F31RSJWZMYE9VH").unwrap(),
        "Les Bleus".to_string(),
        CoachId::from_string("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap(),
        "/teams/pztohgwjq136ggqv4gtr.png".parse().ok()
    );
    return team_to_store;
}