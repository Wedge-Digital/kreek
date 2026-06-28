use crate::app::shared_kernel::common_types::{CoachId, Entity, EntityId};
use crate::app::shared_kernel::id_service::{FakeIdService, IdService};
use crate::app::shared_kernel::team::{BaseTeamInfo, TeamName};
use crate::app::shared_kernel::tier::{CreationBudget, StartingXp, TierName};
use crate::app::team_creation::domain::creation_rules::{CreationRules, CreationTier};
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::app::team_creation::use_cases::create_draft_team::create_draft_team;

fn make_creation_rules() -> CreationRules {
    CreationRules {
        tiers: vec![CreationTier {
            name: TierName::try_new("Debutants").unwrap(),
            budget: CreationBudget(1_000_000),
            start_xp: StartingXp::try_new(0).unwrap(),
            rosters: vec!["Test Roster".to_string()],
        }],
    }
}

#[test]
pub fn assert_team_creation_without_logo_is_ok() {
    let id_service = FakeIdService::new();
    let creator_id = id_service.generate_id();
    let base = BaseTeamInfo::new(
        TeamName::try_new("Les Bleus".to_string()).unwrap(),
        CoachId::try_new("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap(),
        None,
    );
    let team = DraftTeam::new(
        &id_service,
        creator_id,
        base,
        "Coach Test".to_string(),
        "comp-1".to_string(),
        "season-1".to_string(),
        make_creation_rules(),
    );
    assert_eq!(
        team.get_id(),
        EntityId::try_new("01D39ZY06FGSCTVN4T2V9PKHFZ").unwrap()
    );
    assert_eq!(team.competition_id(), "comp-1");
    assert_eq!(team.season_id(), "season-1");
    assert!(!team.creation_rules().tiers.is_empty());
}

#[test]
pub fn assert_new_team_is_given_a_new_id() {
    let id_service = FakeIdService::new();
    let creator_id = id_service.generate_id();
    let base = BaseTeamInfo::new(
        TeamName::try_new("Les Bleus".to_string()).unwrap(),
        CoachId::try_new("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap(),
        None,
    );
    let team = DraftTeam::new(
        &id_service,
        creator_id,
        base,
        "Coach Test".to_string(),
        "comp-1".to_string(),
        "season-1".to_string(),
        make_creation_rules(),
    );
    assert_eq!(
        team.get_id(),
        EntityId::try_new("01D39ZY06FGSCTVN4T2V9PKHFZ").unwrap()
    );
}

#[test]
pub fn assert_factory_method_is_ok() {
    let id_service = FakeIdService::new();
    let coach_id = CoachId::try_new("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap();
    let creator_id = id_service.generate_id();
    let team = create_draft_team(
        &id_service,
        creator_id,
        TeamName::try_new("Les Bleus".to_string()).unwrap(),
        coach_id,
        "Coach Test".to_string(),
        None,
        "comp-1".to_string(),
        "season-1".to_string(),
        make_creation_rules(),
    );
    assert_eq!(
        team.get_id(),
        EntityId::try_new("01D39ZY06FGSCTVN4T2V9PKHFZ").unwrap()
    );
}

#[test]
pub fn assert_team_is_serializable() {
    let id_service = FakeIdService::new();
    let coach_id = CoachId::try_new("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap();
    let creator_id = id_service.generate_id();
    let team = create_draft_team(
        &id_service,
        creator_id,
        TeamName::try_new("Les Bleus".to_string()).unwrap(),
        coach_id,
        "Coach Test".to_string(),
        None,
        "comp-1".to_string(),
        "season-1".to_string(),
        make_creation_rules(),
    );
    let json = serde_json::to_string(&team).unwrap();
    let restored: DraftTeam = serde_json::from_str(&json).unwrap();
    assert_eq!(team.get_id(), restored.get_id());
    assert_eq!(restored.season_id(), "season-1");
    assert_eq!(restored.competition_id(), "comp-1");
}
