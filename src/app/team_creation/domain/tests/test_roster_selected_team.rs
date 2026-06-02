#[cfg(test)]
mod tests {
    use crate::app::shared_kernel::id_service::{FakeIdService, IdService};
    use crate::app::shared_kernel::staff::{
        StaffId, StaffKind, StaffMaxQuantity, StaffName, StaffPrice,
    };
    use crate::app::shared_kernel::team::{BaseTeamInfo, TeamName};
    use crate::app::team_creation::domain::error::DomainError;
    use crate::app::team_creation::domain::roster::{
        CrossLimit, PlayerDefinition, PlayerId, PlayerMaxQuantity, PlayerName, PlayerPrice,
        RerollBasePrice, Roster, RosterId, RosterName,
    };
    use crate::app::team_creation::domain::ruleset::{
        CreationBudget, RosterTier, Ruleset, RulesetId, RulesetName, TierId, TierName,
    };
    use crate::app::team_creation::domain::team_draft::DraftTeam;
    use crate::app::team_creation::domain::team_roster_selected::{
        RosterSelectedTeam, MAX_REROLL_COUNT,
    };
    use crate::app::team_creation::domain::team_ruleset_selected::RulesetSelectedTeam;
    use crate::app::team_creation::domain::team_staff::TeamStaff;
    // -------------------------------------------------------------------------
    // Fixtures
    // -------------------------------------------------------------------------

    fn make_player(id: &str, price: u32, max_quantity: u8) -> PlayerDefinition {
        PlayerDefinition {
            id: PlayerId(id.to_string()),
            name: PlayerName(format!("Player {id}")),
            max_quantity: PlayerMaxQuantity(max_quantity),
            price: PlayerPrice(price),
        }
    }

    fn make_staff(id: &str, price: u32, max_quantity: u8, kind: StaffKind) -> TeamStaff {
        TeamStaff {
            id: StaffId(id.to_string()),
            name: StaffName(format!("Staff {id}")),
            price: StaffPrice(price),
            max_quantity: StaffMaxQuantity(max_quantity),
            kind,
        }
    }

    fn make_roster(
        id: &str,
        players: Vec<PlayerDefinition>,
        staff: Vec<TeamStaff>,
        reroll_price: u32,
    ) -> Roster {
        Roster {
            id: RosterId(id.to_string()),
            name: RosterName(format!("Roster {id}")),
            player_definitions: players,
            allowed_staff: staff,
            cross_limits: vec![],
            reroll_price: RerollBasePrice(reroll_price),
        }
    }

    fn make_ruleset(roster_id: &str, budget: u32) -> Ruleset {
        Ruleset {
            id: RulesetId("ruleset-1".to_string()),
            name: RulesetName("Ruleset 1".to_string()),
            tiers: vec![RosterTier {
                id: TierId("tier-1".to_string()),
                name: TierName("Tier 1".to_string()),
                roster_ids: vec![RosterId(roster_id.to_string())],
                budget: CreationBudget(budget),
            }],
        }
    }

    fn make_roster_selected(roster: Roster, ruleset: Ruleset) -> RosterSelectedTeam {
        use crate::app::shared_kernel::common_types::CoachId;
        use crate::app::team_creation::domain::creation_rules::CreationRules;
        let id_service = FakeIdService::new();
        let creator_id = id_service.generate_id();
        let coach_id = CoachId::try_new("01F8Z3ZQZQZQZQZQZQZQZQZQZQ").unwrap();
        let base = BaseTeamInfo::new(
            TeamName::try_new("Les Bleus".to_string()).unwrap(),
            coach_id,
            None,
        );
        let draft = DraftTeam::new(
            &id_service,
            creator_id,
            base,
            "comp-1".to_string(),
            "season-1".to_string(),
            CreationRules::default(),
        );
        let ruleset_team = RulesetSelectedTeam::new(draft, ruleset);
        RosterSelectedTeam::new(ruleset_team, roster)
    }

    // -------------------------------------------------------------------------
    // hire_player
    // -------------------------------------------------------------------------

    #[test]
    fn hire_player_ok() {
        let player = make_player("p1", 60_000, 2);
        let roster = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        assert!(team.hire_player(player).is_ok());
        assert_eq!(team.hired_players().len(), 1);
    }

    #[test]
    fn hire_player_not_in_roster_returns_error() {
        let player_in = make_player("p1", 60_000, 2);
        let player_out = make_player("p2", 60_000, 2);
        let roster = make_roster("r1", vec![player_in], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        assert_eq!(
            team.hire_player(player_out),
            Err(DomainError::PlayerNotAllowedInRoster)
        );
    }

    #[test]
    fn hire_player_exceeds_max_per_type_returns_error() {
        let player = make_player("p1", 60_000, 1);
        let roster = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        team.hire_player(player.clone()).unwrap();
        assert_eq!(
            team.hire_player(player),
            Err(DomainError::MaxPlayersOfTypeReached)
        );
    }

    #[test]
    fn hire_player_exceeds_total_max_returns_error() {
        let player = make_player("p1", 1_000, 20);
        let roster = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let ruleset = make_ruleset("r1", 10_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        for _ in 0..16 {
            team.hire_player(player.clone()).unwrap();
        }
        assert_eq!(
            team.hire_player(player),
            Err(DomainError::MaxPlayersReached)
        );
    }

    #[test]
    fn hire_player_insufficient_budget_returns_error() {
        let player = make_player("p1", 200_000, 2);
        let roster = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let ruleset = make_ruleset("r1", 100_000);
        let mut team = make_roster_selected(roster, ruleset);

        assert_eq!(
            team.hire_player(player),
            Err(DomainError::InsufficientBudget)
        );
    }

    #[test]
    fn hire_player_cross_limit_exceeded_returns_error() {
        let p1 = make_player("p1", 60_000, 4);
        let p2 = make_player("p2", 60_000, 4);
        let mut roster = make_roster("r1", vec![p1.clone(), p2.clone()], vec![], 50_000);
        roster.cross_limits = vec![CrossLimit {
            limit: 2,
            limited_player_ids: vec![PlayerId("p1".to_string()), PlayerId("p2".to_string())],
        }];
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        team.hire_player(p1.clone()).unwrap();
        team.hire_player(p2.clone()).unwrap();
        assert_eq!(team.hire_player(p1), Err(DomainError::CrossLimitExceeded));
    }

    // -------------------------------------------------------------------------
    // remove_player
    // -------------------------------------------------------------------------

    #[test]
    fn remove_player_ok() {
        let player = make_player("p1", 60_000, 2);
        let roster = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        team.hire_player(player.clone()).unwrap();
        assert!(team.remove_player(&player).is_ok());
        assert_eq!(team.hired_players().len(), 0);
    }

    #[test]
    fn remove_player_not_hired_returns_error() {
        let player = make_player("p1", 60_000, 2);
        let roster = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        assert_eq!(
            team.remove_player(&player),
            Err(DomainError::PlayerNotHired)
        );
    }

    // -------------------------------------------------------------------------
    // buy_staff
    // -------------------------------------------------------------------------

    #[test]
    fn buy_staff_ok() {
        let staff = make_staff("s1", 10_000, 1, StaffKind::Apothecary);
        let roster = make_roster("r1", vec![], vec![staff.clone()], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        assert!(team.buy_staff(staff).is_ok());
        assert_eq!(team.hired_staff().len(), 1);
    }

    #[test]
    fn buy_staff_not_allowed_returns_error() {
        let staff = make_staff("s1", 10_000, 1, StaffKind::Apothecary);
        let roster = make_roster("r1", vec![], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        let errors = team.buy_staff(staff).unwrap_err();
        assert!(errors.contains(&DomainError::StaffNotAllowed));
    }

    #[test]
    fn buy_staff_max_reached_returns_error() {
        let staff = make_staff("s1", 10_000, 1, StaffKind::Apothecary);
        let roster = make_roster("r1", vec![], vec![staff.clone()], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        team.buy_staff(staff.clone()).unwrap();
        let errors = team.buy_staff(staff).unwrap_err();
        assert!(errors.contains(&DomainError::StaffMaxReached));
    }

    #[test]
    fn buy_staff_insufficient_budget_returns_error() {
        let staff = make_staff("s1", 500_000, 1, StaffKind::Apothecary);
        let roster = make_roster("r1", vec![], vec![staff.clone()], 50_000);
        let ruleset = make_ruleset("r1", 100_000);
        let mut team = make_roster_selected(roster, ruleset);

        let errors = team.buy_staff(staff).unwrap_err();
        assert!(errors.contains(&DomainError::InsufficientBudget));
    }

    // -------------------------------------------------------------------------
    // remove_staff
    // -------------------------------------------------------------------------

    #[test]
    fn remove_staff_ok() {
        let staff = make_staff("s1", 10_000, 1, StaffKind::CoachAssistant);
        let roster = make_roster("r1", vec![], vec![staff.clone()], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        team.buy_staff(staff.clone()).unwrap();
        assert!(team.remove_staff(&staff).is_ok());
        assert_eq!(team.hired_staff().len(), 0);
    }

    #[test]
    fn remove_staff_not_purchased_returns_error() {
        let staff = make_staff("s1", 10_000, 1, StaffKind::CoachAssistant);
        let roster = make_roster("r1", vec![], vec![staff.clone()], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        assert_eq!(
            team.remove_staff(&staff),
            Err(DomainError::StaffNotPurchased)
        );
    }

    // -------------------------------------------------------------------------
    // purchase_reroll / remove_reroll
    // -------------------------------------------------------------------------

    #[test]
    fn purchase_reroll_ok() {
        let roster = make_roster("r1", vec![], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        assert!(team.purchase_reroll(3).is_ok());
        assert_eq!(team.reroll_count(), 3);
    }

    #[test]
    fn purchase_reroll_exceeds_max_returns_error() {
        let roster = make_roster("r1", vec![], vec![], 50_000);
        let ruleset = make_ruleset("r1", 10_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        let errors = team.purchase_reroll(MAX_REROLL_COUNT + 1).unwrap_err();
        assert!(errors.contains(&DomainError::MaxRerollsExceeded));
    }

    #[test]
    fn purchase_reroll_insufficient_budget_returns_error() {
        let roster = make_roster("r1", vec![], vec![], 100_000);
        let ruleset = make_ruleset("r1", 50_000);
        let mut team = make_roster_selected(roster, ruleset);

        let errors = team.purchase_reroll(1).unwrap_err();
        assert!(errors.contains(&DomainError::InsufficientRerollBudget));
    }

    #[test]
    fn remove_reroll_decrements_count() {
        let roster = make_roster("r1", vec![], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        team.purchase_reroll(4).unwrap();
        team.remove_reroll(2);
        assert_eq!(team.reroll_count(), 2);
    }

    #[test]
    fn remove_reroll_saturates_at_zero() {
        let roster = make_roster("r1", vec![], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        team.purchase_reroll(2).unwrap();
        team.remove_reroll(10);
        assert_eq!(team.reroll_count(), 0);
    }

    // -------------------------------------------------------------------------
    // choose_roster
    // -------------------------------------------------------------------------

    #[test]
    fn choose_roster_same_id_returns_unchanged_team() {
        let player = make_player("p1", 60_000, 2);
        let roster = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster.clone(), ruleset);

        team.hire_player(player).unwrap();
        let team = team.choose_roster(roster).unwrap();
        assert_eq!(team.hired_players().len(), 1);
    }

    #[test]
    fn choose_roster_new_allowed_roster_resets_purchases() {
        let player = make_player("p1", 60_000, 2);
        let roster_a = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let roster_b = make_roster("r2", vec![], vec![], 50_000);
        let ruleset = Ruleset {
            id: RulesetId("ruleset-1".to_string()),
            name: RulesetName("Ruleset 1".to_string()),
            tiers: vec![RosterTier {
                id: TierId("tier-1".to_string()),
                name: TierName("Tier 1".to_string()),
                roster_ids: vec![RosterId("r1".to_string()), RosterId("r2".to_string())],
                budget: CreationBudget(1_000_000),
            }],
        };
        let mut team = make_roster_selected(roster_a, ruleset);
        team.hire_player(player).unwrap();
        let team = team.choose_roster(roster_b).unwrap();
        assert_eq!(team.hired_players().len(), 0);
    }

    #[test]
    fn choose_roster_not_in_ruleset_returns_error() {
        let roster_a = make_roster("r1", vec![], vec![], 50_000);
        let roster_b = make_roster("r2", vec![], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let team = make_roster_selected(roster_a, ruleset);

        assert_eq!(
            team.choose_roster(roster_b),
            Err(DomainError::RosterNotAllowed)
        );
    }

    // -------------------------------------------------------------------------
    // remaining_budget
    // -------------------------------------------------------------------------

    #[test]
    fn remaining_budget_decreases_after_hire() {
        let player = make_player("p1", 80_000, 2);
        let roster = make_roster("r1", vec![player.clone()], vec![], 50_000);
        let ruleset = make_ruleset("r1", 1_000_000);
        let mut team = make_roster_selected(roster, ruleset);

        team.hire_player(player).unwrap();
        assert_eq!(team.remaining_budget().unwrap(), 920_000);
    }
}
