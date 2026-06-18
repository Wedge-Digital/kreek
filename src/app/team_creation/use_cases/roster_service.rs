use crate::app::shared_kernel::staff::{
    StaffId, StaffKind, StaffMaxQuantity, StaffName, StaffPrice,
};
use crate::app::team_creation::domain::roster::{
    PlayerDefinition, PlayerId, PlayerMaxQuantity, PlayerName, PlayerPrice, RerollBasePrice, Roster,
    RosterId, RosterName,
};
use crate::app::team_creation::domain::team_staff::TeamStaff;
use crate::app::team_creation::ports::IReferenceDataPort;

pub struct RosterMetadata {
    pub leagues: Vec<String>,
    pub special_rules: Vec<String>,
}

pub fn load_roster(roster_uid: &str, ref_data: &dyn IReferenceDataPort) -> Option<Roster> {
    let def = ref_data.find_roster_definition(roster_uid)?;

    let player_definitions = def
        .available_players
        .iter()
        .map(|p| PlayerDefinition {
            id: PlayerId(p.uid.clone()),
            name: PlayerName(p.position_name.clone()),
            max_quantity: PlayerMaxQuantity(p.max_quantity),
            price: PlayerPrice(p.cost / 1000),
        })
        .collect();

    let all_staff = ref_data.list_staff_definitions();
    let mut allowed_staff: Vec<TeamStaff> = def
        .allowed_staff_uids
        .iter()
        .filter_map(|uid| {
            all_staff
                .iter()
                .find(|s| s.uid == *uid)
                .map(|s| TeamStaff {
                    id: StaffId(s.uid.clone()),
                    name: StaffName(s.name.clone()),
                    price: StaffPrice(s.price),
                    max_quantity: StaffMaxQuantity(s.max_quantity),
                    kind: staff_kind(&s.uid),
                })
        })
        .collect();

    if !allowed_staff.iter().any(|s| s.id.0 == "FAN_FACTOR") {
        if let Some(ff) = all_staff.iter().find(|s| s.uid == "FAN_FACTOR") {
            allowed_staff.push(TeamStaff {
                id: StaffId(ff.uid.clone()),
                name: StaffName(ff.name.clone()),
                price: StaffPrice(ff.price),
                max_quantity: StaffMaxQuantity(ff.max_quantity),
                kind: StaffKind::FansFactor,
            });
        }
    }

    Some(Roster {
        id: RosterId(def.uid.clone()),
        name: RosterName(def.name.clone()),
        player_definitions,
        allowed_staff,
        cross_limits: vec![],
        reroll_price: RerollBasePrice(def.reroll_cost / 1000),
    })
}

pub fn roster_metadata(
    roster_uid: &str,
    ref_data: &dyn IReferenceDataPort,
) -> Option<RosterMetadata> {
    let def = ref_data.find_roster_definition(roster_uid)?;
    Some(RosterMetadata {
        leagues: def.leagues,
        special_rules: def.special_rules,
    })
}

fn staff_kind(uid: &str) -> StaffKind {
    match uid {
        "APOTHECARY" => StaffKind::Apothecary,
        "CHEERLEADERS" => StaffKind::Cheerleaders,
        "COACH_ASSISTANTS" => StaffKind::CoachAssistant,
        "FAN_FACTOR" => StaffKind::FansFactor,
        _ => StaffKind::CoachAssistant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::team_creation::ports::{
        PlayerPositionDefinition, RosterDefinition, SkillDefinition, StaffDefinition,
    };

    struct FakeRefData;

    impl IReferenceDataPort for FakeRefData {
        fn find_roster_definition(&self, uid: &str) -> Option<RosterDefinition> {
            if uid != "LIZARDMEN" {
                return None;
            }
            Some(RosterDefinition {
                uid: "LIZARDMEN".into(),
                name: "Hommes-Lézards".into(),
                reroll_cost: 70000,
                available_players: vec![PlayerPositionDefinition {
                    uid: "SKINK".into(),
                    position_name: "Skink".into(),
                    cost: 60000,
                    max_quantity: 12,
                    ma: 8,
                    st: 2,
                    ag: 3,
                    pa: 4,
                    av: 8,
                    skills: vec![SkillDefinition {
                        uid: "DODGE".into(),
                        name: "Esquive".into(),
                    }],
                }],
                allowed_staff_uids: vec!["APOTHECARY".into()],
                leagues: vec!["WOODLAND".into()],
                special_rules: vec!["LUSTRIAN_SUPERLEAGUE".into()],
            })
        }

        fn list_staff_definitions(&self) -> Vec<StaffDefinition> {
            vec![
                StaffDefinition {
                    uid: "APOTHECARY".into(),
                    name: "Apothicaire".into(),
                    price: 50,
                    max_quantity: 1,
                },
                StaffDefinition {
                    uid: "FAN_FACTOR".into(),
                    name: "Facteur de fans".into(),
                    price: 10,
                    max_quantity: 6,
                },
            ]
        }

        fn resolve_skill_cost(&self, _: &str, _: &str, _: &str) -> Option<crate::app::team_creation::ports::SkillCostResult> {
            Some(crate::app::team_creation::ports::SkillCostResult { spp_cost: 3 })
        }

        fn resolve_base_skills(&self, _: &str) -> Vec<String> { vec!["Esquive".into()] }

        fn skill_pricing_level_1(&self) -> Option<crate::app::team_creation::ports::SkillPricingDefinition> {
            Some(crate::app::team_creation::ports::SkillPricingDefinition {
                chosen_primary: 3, chosen_secondary: 6, random: 2,
            })
        }
    }

    #[test]
    fn load_roster_returns_none_for_unknown() {
        assert!(load_roster("UNKNOWN", &FakeRefData).is_none());
    }

    #[test]
    fn load_roster_builds_valid_roster() {
        let roster = load_roster("LIZARDMEN", &FakeRefData).unwrap();
        assert_eq!(roster.id.0, "LIZARDMEN");
        assert_eq!(roster.name.0, "Hommes-Lézards");
        assert_eq!(roster.reroll_price.0, 70);
        assert_eq!(roster.player_definitions.len(), 1);
        assert_eq!(roster.player_definitions[0].price.0, 60);
    }

    #[test]
    fn load_roster_includes_fan_factor() {
        let roster = load_roster("LIZARDMEN", &FakeRefData).unwrap();
        assert!(roster.allowed_staff.iter().any(|s| s.id.0 == "FAN_FACTOR"));
        assert!(roster.allowed_staff.iter().any(|s| s.id.0 == "APOTHECARY"));
        assert_eq!(roster.allowed_staff.len(), 2);
    }

    #[test]
    fn roster_metadata_returns_leagues_and_rules() {
        let meta = roster_metadata("LIZARDMEN", &FakeRefData).unwrap();
        assert_eq!(meta.leagues, vec!["WOODLAND"]);
        assert_eq!(meta.special_rules, vec!["LUSTRIAN_SUPERLEAGUE"]);
    }
}
