use crate::app::shared_kernel::bloodbowl::ids::RosterId;
use crate::app::shared_kernel::bloodbowl::staff::{
    StaffId, StaffKind, StaffMaxQuantity, StaffName, StaffPrice,
};
use crate::app::team_creation::domain::roster::{
    PlayerDefinition, PlayerId, PlayerMaxQuantity, PlayerName, PlayerPrice, RerollBasePrice,
    Roster, RosterName,
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
        .filter_map(|p| {
            Some(PlayerDefinition {
                id: PlayerId(p.uid.clone()),
                name: PlayerName::try_new(p.position_name.clone()).ok()?,
                max_quantity: PlayerMaxQuantity::try_new(p.max_quantity).ok()?,
                price: PlayerPrice::try_new(p.cost).ok()?,
            })
        })
        .collect();

    let all_staff = ref_data.list_staff_definitions();
    let mut allowed_staff: Vec<TeamStaff> = def
        .allowed_staff_uids
        .iter()
        .filter_map(|uid| {
            all_staff.iter().find(|s| s.uid == *uid).map(|s| TeamStaff {
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
        name: RosterName::try_new(def.name.clone()).ok()?,
        player_definitions,
        allowed_staff,
        cross_limits: vec![],
        reroll_price: RerollBasePrice::try_new(def.reroll_cost).ok()?,
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

/// Un roster à choix multiple de ligues doit avoir une ligue affectée
/// pour pouvoir terminer la construction. Un roster à ligue unique est
/// auto-affecté pendant le build : ce cas ne déclenche jamais ce blocage.
pub fn league_selection_missing(
    roster_uid: &str,
    league_already_set: bool,
    ref_data: &dyn IReferenceDataPort,
) -> bool {
    if league_already_set {
        return false;
    }
    roster_metadata(roster_uid, ref_data)
        .map(|m| m.leagues.len() > 1)
        .unwrap_or(false)
}

/// Un roster à choix de règle spéciale (FAVOURED_OF_CHOOSE_*, ex. Chaos
/// Renégats/Élus) doit avoir une règle affectée pour pouvoir terminer la
/// construction. Un roster à règle(s) fixe(s) — ou sans règle — ne déclenche
/// jamais ce blocage.
pub fn special_rule_selection_missing(
    roster_uid: &str,
    special_rule_already_set: bool,
    ref_data: &dyn IReferenceDataPort,
) -> bool {
    if special_rule_already_set {
        return false;
    }
    roster_metadata(roster_uid, ref_data)
        .map(|m| {
            m.special_rules
                .iter()
                .any(|r| r.starts_with("FAVOURED_OF_CHOOSE_"))
        })
        .unwrap_or(false)
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
            match uid {
                "LIZARDMEN" => Some(RosterDefinition {
                    uid: "LIZARDMEN".into(),
                    name: "Hommes-Lézards".into(),
                    reroll_cost: 70,
                    available_players: vec![PlayerPositionDefinition {
                        uid: "SKINK".into(),
                        position_name: "Skink".into(),
                        cost: 60,
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
                }),
                "CHAOS_DWARF" => Some(RosterDefinition {
                    uid: "CHAOS_DWARF".into(),
                    name: "Nain du Chaos".into(),
                    reroll_cost: 70,
                    available_players: vec![],
                    allowed_staff_uids: vec!["APOTHECARY".into()],
                    leagues: vec!["BADLANDS_BRAWL".into(), "CHAOS_CLASH".into()],
                    special_rules: vec!["FAVOURED_OF_HASHUT".into()],
                }),
                "CHAOS_RENEGADE" => Some(RosterDefinition {
                    uid: "CHAOS_RENEGADE".into(),
                    name: "Renégats du Chaos".into(),
                    reroll_cost: 60,
                    available_players: vec![],
                    allowed_staff_uids: vec![],
                    leagues: vec!["CHAOS_CLASH".into()],
                    special_rules: vec![
                        "FAVOURED_OF_CHOOSE_EITHER_KHORNE_NURGLE_SLAANESH_TZEENTCH_OR_UNDIVIDED"
                            .into(),
                    ],
                }),
                _ => None,
            }
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

        fn resolve_skill_cost(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Option<crate::app::team_creation::ports::SkillCostResult> {
            Some(crate::app::team_creation::ports::SkillCostResult { spp_cost: 3 })
        }

        fn resolve_skill_name(&self, uid: &str) -> Option<String> {
            if uid == "DODGE" {
                Some("Esquive".into())
            } else {
                None
            }
        }

        fn resolve_base_skills(&self, _: &str) -> Vec<String> {
            vec!["Esquive".into()]
        }

        fn skill_pricing_level_1(
            &self,
        ) -> Option<crate::app::team_creation::ports::SkillPricingDefinition> {
            Some(crate::app::team_creation::ports::SkillPricingDefinition {
                chosen_primary: 3,
                chosen_secondary: 6,
                random: 2,
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
        assert_eq!(roster.name.as_ref(), "Hommes-Lézards");
        assert_eq!(roster.reroll_price.into_inner(), 70);
        assert_eq!(roster.player_definitions.len(), 1);
        assert_eq!(roster.player_definitions[0].price.into_inner(), 60);
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

    #[test]
    fn league_selection_missing_false_for_single_league_roster() {
        // LIZARDMEN n'a qu'une ligue : jamais bloquant, même sans league_id affecté.
        assert!(!league_selection_missing("LIZARDMEN", false, &FakeRefData));
    }

    #[test]
    fn league_selection_missing_true_for_multi_league_roster_without_selection() {
        // CHAOS_DWARF a 2 ligues possibles : bloquant tant qu'aucune n'est choisie.
        assert!(league_selection_missing("CHAOS_DWARF", false, &FakeRefData));
    }

    #[test]
    fn league_selection_missing_false_once_league_selected() {
        assert!(!league_selection_missing("CHAOS_DWARF", true, &FakeRefData));
    }

    #[test]
    fn league_selection_missing_false_for_unknown_roster() {
        assert!(!league_selection_missing("UNKNOWN", false, &FakeRefData));
    }

    #[test]
    fn special_rule_selection_missing_false_for_fixed_rule_roster() {
        // CHAOS_DWARF a une règle fixe (FAVOURED_OF_HASHUT), pas de choix : jamais bloquant.
        assert!(!special_rule_selection_missing(
            "CHAOS_DWARF",
            false,
            &FakeRefData
        ));
    }

    #[test]
    fn special_rule_selection_missing_true_for_choice_roster_without_selection() {
        assert!(special_rule_selection_missing(
            "CHAOS_RENEGADE",
            false,
            &FakeRefData
        ));
    }

    #[test]
    fn special_rule_selection_missing_false_once_selected() {
        assert!(!special_rule_selection_missing(
            "CHAOS_RENEGADE",
            true,
            &FakeRefData
        ));
    }

    #[test]
    fn special_rule_selection_missing_false_for_unknown_roster() {
        assert!(!special_rule_selection_missing(
            "UNKNOWN",
            false,
            &FakeRefData
        ));
    }
}
