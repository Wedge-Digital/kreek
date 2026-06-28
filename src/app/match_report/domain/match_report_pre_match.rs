use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::value_objects::{
    ActionId, ActionPlayer, AllowedInducementSpec, D3Roll, InducementPurchase, InducementQty,
    MatchAction, MatchActionType, MatchReportOrigin, TeamSide, TeamValue, TempPlayer,
};
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::team::TeamId;
use crate::app::match_report::domain::value_objects::TurnNumber;

#[derive(Debug, Clone)]
pub struct MatchReportPreMatch {
    pub id: MatchReportId,
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub origin: MatchReportOrigin,
    pub pairing_id: Option<String>,
    pub home_fan_roll: Option<D3Roll>,
    pub away_fan_roll: Option<D3Roll>,
    pub home_team_value: Option<TeamValue>,
    pub away_team_value: Option<TeamValue>,
    pub home_inducements: Option<Vec<InducementPurchase>>,
    pub away_inducements: Option<Vec<InducementPurchase>>,
    pub star_engagements: Vec<(TeamId, InducementId)>,
    pub home_temp_players: Vec<TempPlayer>,
    pub away_temp_players: Vec<TempPlayer>,
    pub home_actions: Vec<MatchAction>,
    pub away_actions: Vec<MatchAction>,
    pub version: u64,
}

impl MatchReportPreMatch {
    pub fn record_fan_factor(
        &self,
        home_fan_roll: D3Roll,
        away_fan_roll: D3Roll,
        recorded_by: CoachId,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::FanFactorRecorded { home_fan_roll, away_fan_roll, recorded_by };
        let mut updated = self.clone();
        updated.home_fan_roll = Some(home_fan_roll);
        updated.away_fan_roll = Some(away_fan_roll);
        updated.version += 1;
        (updated, event)
    }

    pub fn record_team_values(
        &self,
        home_tv: TeamValue,
        away_tv: TeamValue,
        recorded_by: CoachId,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::TeamValuesRecorded { home_team_value: home_tv, away_team_value: away_tv, recorded_by };
        let mut updated = self.clone();
        updated.home_team_value = Some(home_tv);
        updated.away_team_value = Some(away_tv);
        updated.version += 1;
        (updated, event)
    }

    pub fn topdog_team_id(&self) -> &TeamId {
        let home_tv = self.home_team_value.as_ref().expect("TV not recorded");
        let away_tv = self.away_team_value.as_ref().expect("TV not recorded");
        if away_tv > home_tv { &self.away_team_id } else { &self.home_team_id }
    }

    pub fn underdog_team_id(&self) -> &TeamId {
        if self.topdog_team_id() == &self.home_team_id { &self.away_team_id } else { &self.home_team_id }
    }

    pub fn topdog_spending(&self) -> u32 {
        let topdog = self.topdog_team_id().clone();
        let purchases = if topdog == self.home_team_id {
            self.home_inducements.as_deref()
        } else {
            self.away_inducements.as_deref()
        };
        purchases.unwrap_or(&[]).iter().map(|p| p.total_cost()).sum()
    }

    pub fn inducement_budget_for(&self, team_id: &TeamId, treasury: u32) -> u32 {
        if team_id == self.topdog_team_id() {
            treasury
        } else {
            let tv_diff = self.home_team_value.unwrap().into_inner().abs_diff(self.away_team_value.unwrap().into_inner());
            tv_diff + self.topdog_spending() + treasury.min(50)
        }
    }

    pub fn record_inducements(
        &self,
        team_id: &TeamId,
        purchases: &[(InducementId, u8)],
        budget: u32,
        allowed_specs: &[AllowedInducementSpec],
        opponent_star_uids: &[InducementId],
        recorded_by: CoachId,
    ) -> Result<(Self, Vec<MatchReportDomainEvent>), DomainError> {
        if self.inducements_for(team_id).is_some() {
            return Err(DomainError::InducementsAlreadyRecorded);
        }
        validate_purchases(purchases, allowed_specs, opponent_star_uids, budget)?;
        let purchase_list = build_purchase_list(purchases, allowed_specs);
        let events = build_inducement_events(team_id, &purchase_list, allowed_specs, recorded_by);
        let mut updated = self.clone();
        set_inducements_for(&mut updated, team_id, purchase_list);
        updated.version += events.len() as u64;
        Ok((updated, events))
    }

    pub fn is_inducements_phase_complete(&self) -> bool {
        self.home_inducements.is_some() && self.away_inducements.is_some()
    }

    fn inducements_for(&self, team_id: &TeamId) -> Option<&Vec<InducementPurchase>> {
        if team_id == &self.home_team_id { self.home_inducements.as_ref() } else { self.away_inducements.as_ref() }
    }

    pub fn init_temp_players(
        &self,
        team_id: &TeamId,
        players: Vec<TempPlayer>,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::TempPlayersInitialized { team_id: team_id.clone(), players: players.clone() };
        let mut updated = self.clone();
        if team_id == &updated.home_team_id {
            updated.home_temp_players = players;
        } else {
            updated.away_temp_players = players;
        }
        updated.version += 1;
        (updated, event)
    }

    pub fn reset_temp_players(&self, team_id: &TeamId) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::TempPlayersReset { team_id: team_id.clone() };
        let mut updated = self.clone();
        if team_id == &updated.home_team_id {
            updated.home_temp_players = vec![];
        } else {
            updated.away_temp_players = vec![];
        }
        updated.version += 1;
        (updated, event)
    }

    pub fn record_action(
        &self,
        team_side: TeamSide,
        turn: TurnNumber,
        player: ActionPlayer,
        action: MatchActionType,
        player_display_name: String,
        action_id: ActionId,
        recorded_by: CoachId,
    ) -> (Self, MatchReportDomainEvent) {
        let event = MatchReportDomainEvent::ActionRecorded {
            action_id: action_id.clone(),
            team_side,
            turn,
            player: player.clone(),
            action: action.clone(),
            player_display_name: player_display_name.clone(),
            recorded_by,
        };
        let entry = MatchAction { id: action_id, turn, player, action, player_display_name };
        let mut updated = self.clone();
        match team_side {
            TeamSide::Home => updated.home_actions.push(entry),
            TeamSide::Away => updated.away_actions.push(entry),
        }
        updated.version += 1;
        (updated, event)
    }

    pub fn delete_action(
        &self,
        action_id: &ActionId,
        deleted_by: CoachId,
    ) -> Result<(Self, MatchReportDomainEvent), DomainError> {
        let team_side = if self.home_actions.iter().any(|a| &a.id == action_id) {
            TeamSide::Home
        } else if self.away_actions.iter().any(|a| &a.id == action_id) {
            TeamSide::Away
        } else {
            return Err(DomainError::ActionNotFound(action_id.0.clone()));
        };
        let event = MatchReportDomainEvent::ActionDeleted {
            action_id: action_id.clone(),
            team_side,
            deleted_by,
        };
        let mut updated = self.clone();
        match team_side {
            TeamSide::Home => updated.home_actions.retain(|a| &a.id != action_id),
            TeamSide::Away => updated.away_actions.retain(|a| &a.id != action_id),
        }
        updated.version += 1;
        Ok((updated, event))
    }

    pub fn temp_players_for(&self, side: TeamSide) -> &[TempPlayer] {
        match side {
            TeamSide::Home => &self.home_temp_players,
            TeamSide::Away => &self.away_temp_players,
        }
    }

    pub fn star_player_uids_for(&self, team_id: &TeamId) -> Vec<InducementId> {
        self.star_engagements.iter()
            .filter(|(tid, _)| tid == team_id)
            .map(|(_, uid)| uid.clone())
            .collect()
    }

    pub fn purchases_for(&self, team_id: &TeamId) -> &[InducementPurchase] {
        self.inducements_for(team_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn actions_for(&self, side: TeamSide) -> &[MatchAction] {
        match side {
            TeamSide::Home => &self.home_actions,
            TeamSide::Away => &self.away_actions,
        }
    }

    pub fn from_draft(draft: MatchReportDraft) -> Self {
        Self {
            id: draft.id,
            space_id: draft.space_id,
            competition_id: draft.competition_id,
            season_id: draft.season_id,
            round_id: draft.round_id,
            home_team_id: draft.home_team_id,
            away_team_id: draft.away_team_id,
            created_by: draft.created_by,
            origin: draft.origin,
            pairing_id: draft.pairing_id,
            home_fan_roll: None,
            away_fan_roll: None,
            home_team_value: None,
            away_team_value: None,
            home_inducements: None,
            away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            version: draft.version + 1,
        }
    }
}

fn validate_purchases(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
    opponent_star_uids: &[InducementId],
    budget: u32,
) -> Result<(), DomainError> {
    validate_max_qty(purchases, allowed_specs)?;
    validate_star_player_limit(purchases, allowed_specs)?;
    validate_star_player_conflict(purchases, allowed_specs, opponent_star_uids)?;
    validate_budget(purchases, allowed_specs, budget)
}

fn validate_max_qty(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
) -> Result<(), DomainError> {
    for (uid, qty) in purchases {
        if let Some(spec) = allowed_specs.iter().find(|s| &s.uid == uid) {
            if *qty > spec.max_qty.into_inner() {
                return Err(DomainError::MaxQtyExceeded { uid: uid.0.clone(), qty: *qty, max_qty: spec.max_qty.into_inner() });
            }
        }
    }
    Ok(())
}

fn validate_star_player_limit(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
) -> Result<(), DomainError> {
    let star_count = purchases.iter()
        .filter(|(uid, _)| allowed_specs.iter().any(|s| &s.uid == uid && s.is_star_player.0))
        .count();
    if star_count > 2 { Err(DomainError::StarPlayerLimitExceeded) } else { Ok(()) }
}

fn validate_star_player_conflict(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
    opponent_star_uids: &[InducementId],
) -> Result<(), DomainError> {
    for (uid, _) in purchases {
        let is_star = allowed_specs.iter().any(|s| &s.uid == uid && s.is_star_player.0);
        if is_star && opponent_star_uids.contains(uid) {
            return Err(DomainError::StarPlayerConflict { uid: uid.0.clone() });
        }
    }
    Ok(())
}

fn validate_budget(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
    budget: u32,
) -> Result<(), DomainError> {
    let spent: u32 = purchases.iter()
        .filter_map(|(uid, qty)| allowed_specs.iter().find(|s| &s.uid == uid).map(|s| s.unit_cost.into_inner() * (*qty as u32)))
        .sum();
    if spent > budget { Err(DomainError::BudgetExceeded { spent, budget }) } else { Ok(()) }
}

fn build_purchase_list(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
) -> Vec<InducementPurchase> {
    purchases.iter()
        .filter_map(|(uid, qty)| {
            allowed_specs.iter().find(|s| &s.uid == uid).map(|spec| InducementPurchase {
                uid: uid.clone(),
                qty: InducementQty::try_new(*qty).expect("qty validated at IO boundary"),
                unit_cost: spec.unit_cost,
            })
        })
        .collect()
}

fn build_inducement_events(
    team_id: &TeamId,
    purchase_list: &[InducementPurchase],
    allowed_specs: &[AllowedInducementSpec],
    recorded_by: CoachId,
) -> Vec<MatchReportDomainEvent> {
    let mut events = vec![MatchReportDomainEvent::InducementsRecorded {
        team_id: team_id.clone(),
        purchases: purchase_list.to_vec(),
        recorded_by: recorded_by.clone(),
    }];
    for p in purchase_list {
        if allowed_specs.iter().any(|s| s.uid == p.uid && s.is_star_player.0) {
            events.push(MatchReportDomainEvent::StarPlayerEngaged {
                team_id: team_id.clone(),
                star_player_uid: p.uid.clone(),
                recorded_by: recorded_by.clone(),
            });
        }
    }
    events
}

fn set_inducements_for(pm: &mut MatchReportPreMatch, team_id: &TeamId, purchases: Vec<InducementPurchase>) {
    if team_id == &pm.home_team_id {
        pm.home_inducements = Some(purchases);
    } else {
        pm.away_inducements = Some(purchases);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{
        InducementCost, InducementQty, IsStarPlayer, MatchReportOrigin,
    };
    use crate::app::shared_kernel::common_types::{
        CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
    };

    fn make_pm(home_tv: u32, away_tv: u32) -> MatchReportPreMatch {
        let home_id = TeamId::new();
        let away_id = TeamId::new();
        MatchReportPreMatch {
            id: MatchReportId::new(), space_id: SpaceId::new(),
            competition_id: CompetitionId::new(), season_id: SeasonId::new(),
            round_id: RoundId::new(), home_team_id: home_id, away_team_id: away_id,
            created_by: CoachId::new(), origin: MatchReportOrigin::Manual, pairing_id: None,
            home_fan_roll: None, away_fan_roll: None,
            home_team_value: Some(TeamValue::try_new(home_tv).unwrap()), away_team_value: Some(TeamValue::try_new(away_tv).unwrap()),
            home_inducements: None, away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![], away_temp_players: vec![],
            home_actions: vec![], away_actions: vec![],
            version: 1,
        }
    }

    fn spec(uid: &str, max_qty: u8, unit_cost: u32, is_star: bool) -> AllowedInducementSpec {
        AllowedInducementSpec {
            uid: InducementId(uid.to_string()),
            max_qty: InducementQty::try_new(max_qty).unwrap(),
            unit_cost: InducementCost::try_new(unit_cost).unwrap(),
            is_star_player: IsStarPlayer(is_star),
        }
    }

    #[test]
    fn topdog_is_home_when_tv_equal() {
        let pm = make_pm(1000, 1000);
        assert_eq!(pm.topdog_team_id(), &pm.home_team_id);
    }

    #[test]
    fn topdog_is_away_when_away_higher_tv() {
        let pm = make_pm(1000, 1100);
        assert_eq!(pm.topdog_team_id(), &pm.away_team_id);
    }

    #[test]
    fn topdog_budget_equals_treasury() {
        let pm = make_pm(1000, 1000);
        assert_eq!(pm.inducement_budget_for(&pm.home_team_id, 80), 80);
    }

    #[test]
    fn underdog_budget_includes_tv_diff_and_topdog_spending() {
        // diff = 100 kPo, treasury = 0
        let pm = make_pm(1100, 1000);
        assert_eq!(pm.inducement_budget_for(&pm.away_team_id, 0), 100);
    }

    #[test]
    fn underdog_budget_caps_treasury_at_50k() {
        // diff = 100 kPo, treasury = 80 kPo → capped at 50 kPo → total = 150 kPo
        let pm = make_pm(1100, 1000);
        assert_eq!(pm.inducement_budget_for(&pm.away_team_id, 80), 150);
    }

    #[test]
    fn underdog_budget_uses_full_treasury_when_below_50k() {
        // diff = 100 kPo, treasury = 30 kPo < 50 kPo cap → total = 130 kPo
        let pm = make_pm(1100, 1000);
        assert_eq!(pm.inducement_budget_for(&pm.away_team_id, 30), 130);
    }

    #[test]
    fn topdog_spending_zero_before_purchase() {
        let pm = make_pm(1100, 1000);
        assert_eq!(pm.topdog_spending(), 0);
    }

    #[test]
    fn topdog_spending_zero_when_passed() {
        let mut pm = make_pm(1100, 1000);
        pm.home_inducements = Some(vec![]);
        assert_eq!(pm.topdog_spending(), 0);
    }

    #[test]
    fn record_inducements_fails_on_budget_exceeded() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("BRIBE", 2, 100, false)];
        let result = pm.record_inducements(&pm.home_team_id.clone(), &[(InducementId("BRIBE".into()), 1)], 50, &specs, &[], CoachId::new());
        assert!(matches!(result, Err(DomainError::BudgetExceeded { .. })));
    }

    #[test]
    fn record_inducements_fails_on_max_qty_exceeded() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("BRIBE", 1, 10, false)];
        let result = pm.record_inducements(&pm.home_team_id.clone(), &[(InducementId("BRIBE".into()), 2)], 100, &specs, &[], CoachId::new());
        assert!(matches!(result, Err(DomainError::MaxQtyExceeded { .. })));
    }

    #[test]
    fn record_inducements_fails_when_star_player_limit_exceeded() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("SP1", 1, 10_000, true), spec("SP2", 1, 10_000, true), spec("SP3", 1, 10_000, true)];
        let result = pm.record_inducements(&pm.home_team_id.clone(), &[
            (InducementId("SP1".into()), 1), (InducementId("SP2".into()), 1), (InducementId("SP3".into()), 1),
        ], 1_000_000, &specs, &[], CoachId::new());
        assert!(matches!(result, Err(DomainError::StarPlayerLimitExceeded)));
    }

    #[test]
    fn record_inducements_fails_on_star_player_conflict() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("SP1", 1, 10_000, true)];
        let opponent = vec![InducementId("SP1".into())];
        let result = pm.record_inducements(&pm.home_team_id.clone(), &[(InducementId("SP1".into()), 1)], 1_000_000, &specs, &opponent, CoachId::new());
        assert!(matches!(result, Err(DomainError::StarPlayerConflict { .. })));
    }

    #[test]
    fn record_inducements_with_empty_purchases_succeeds() {
        let pm = make_pm(1000, 1000);
        let result = pm.record_inducements(&pm.home_team_id.clone(), &[], 0, &[], &[], CoachId::new());
        assert!(result.is_ok());
        let (updated, events) = result.unwrap();
        assert!(updated.home_inducements.as_ref().unwrap().is_empty());
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn record_inducements_emits_star_player_engaged_per_star() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("SP1", 1, 10_000, true), spec("SP2", 1, 10_000, true)];
        let (_, events) = pm.record_inducements(&pm.home_team_id.clone(), &[
            (InducementId("SP1".into()), 1), (InducementId("SP2".into()), 1),
        ], 1_000_000, &specs, &[], CoachId::new()).unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[1], MatchReportDomainEvent::StarPlayerEngaged { .. }));
        assert!(matches!(&events[2], MatchReportDomainEvent::StarPlayerEngaged { .. }));
    }

    #[test]
    fn record_inducements_no_star_player_engaged_when_none_hired() {
        let pm = make_pm(1000, 1000);
        let specs = vec![spec("BRIBE", 2, 10_000, false)];
        let (_, events) = pm.record_inducements(&pm.home_team_id.clone(), &[(InducementId("BRIBE".into()), 1)], 1_000_000, &specs, &[], CoachId::new()).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn is_inducements_phase_complete_when_both_recorded() {
        let mut pm = make_pm(1000, 1000);
        pm.home_inducements = Some(vec![]);
        pm.away_inducements = Some(vec![]);
        assert!(pm.is_inducements_phase_complete());
    }

    #[test]
    fn is_inducements_phase_not_complete_when_only_one_recorded() {
        let mut pm = make_pm(1000, 1000);
        pm.home_inducements = Some(vec![]);
        assert!(!pm.is_inducements_phase_complete());
    }

    #[test]
    fn record_inducements_fails_when_already_recorded() {
        let mut pm = make_pm(1000, 1000);
        pm.home_inducements = Some(vec![]);
        let result = pm.record_inducements(&pm.home_team_id.clone(), &[], 0, &[], &[], CoachId::new());
        assert!(matches!(result, Err(DomainError::InducementsAlreadyRecorded)));
    }

    // ── step3-4 : temp players ────────────────────────────────────────────────

    use crate::app::match_report::domain::value_objects::{TempPlayerId, TempPlayerKind};

    fn make_journalier(pm: &MatchReportPreMatch) -> TempPlayer {
        TempPlayer {
            id: TempPlayerId("tp-01".to_string()),
            team_id: pm.home_team_id.clone(),
            kind: TempPlayerKind::Journalier { position_uid: "LIN".to_string() },
            display_name: None,
        }
    }

    #[test]
    fn init_temp_players_sets_list() {
        let pm = make_pm(1000, 1000);
        let player = make_journalier(&pm);
        let (updated, event) = pm.init_temp_players(&pm.home_team_id.clone(), vec![player]);
        assert_eq!(updated.home_temp_players.len(), 1);
        assert!(updated.away_temp_players.is_empty());
        assert!(matches!(event, MatchReportDomainEvent::TempPlayersInitialized { .. }));
        assert_eq!(updated.version, pm.version + 1);
    }

    #[test]
    fn reset_temp_players_clears_list() {
        let pm = make_pm(1000, 1000);
        let player = make_journalier(&pm);
        let (with_players, _) = pm.init_temp_players(&pm.home_team_id.clone(), vec![player]);
        let (reset, event) = with_players.reset_temp_players(&pm.home_team_id.clone());
        assert!(reset.home_temp_players.is_empty());
        assert!(matches!(event, MatchReportDomainEvent::TempPlayersReset { .. }));
    }

    // ── step3-4 : actions ─────────────────────────────────────────────────────

    use crate::app::match_report::domain::value_objects::{ActionId, ActionPlayer, MatchActionType, TeamSide, TurnNumber};
    use crate::app::shared_kernel::common_types::PlayerId;

    fn make_action(pm: &MatchReportPreMatch, side: TeamSide) -> (MatchReportPreMatch, MatchReportDomainEvent) {
        pm.record_action(
            side,
            TurnNumber::try_new(3).unwrap(),
            ActionPlayer::Regular(PlayerId::new()),
            MatchActionType::Touchdown,
            "Jean Dupont (#5)".to_string(),
            ActionId("act-01".to_string()),
            CoachId::new(),
        )
    }

    #[test]
    fn record_action_pushes_to_home_actions() {
        let pm = make_pm(1000, 1000);
        let (updated, _) = make_action(&pm, TeamSide::Home);
        assert_eq!(updated.home_actions.len(), 1);
        assert!(updated.away_actions.is_empty());
    }

    #[test]
    fn record_action_pushes_to_away_actions() {
        let pm = make_pm(1000, 1000);
        let (updated, _) = make_action(&pm, TeamSide::Away);
        assert_eq!(updated.away_actions.len(), 1);
        assert!(updated.home_actions.is_empty());
    }

    #[test]
    fn record_two_actions_same_player_same_turn() {
        let pm = make_pm(1000, 1000);
        let (pm2, _) = make_action(&pm, TeamSide::Home);
        let (pm3, _) = make_action(&pm2, TeamSide::Home);
        assert_eq!(pm3.home_actions.len(), 2);
    }

    #[test]
    fn record_two_mvp_same_team() {
        let pm = make_pm(1000, 1000);
        let (pm2, _) = pm.record_action(TeamSide::Home, TurnNumber::try_new(1).unwrap(), ActionPlayer::Regular(PlayerId::new()), MatchActionType::Mvp, "A".to_string(), ActionId("a1".to_string()), CoachId::new());
        let (pm3, _) = pm2.record_action(TeamSide::Home, TurnNumber::try_new(1).unwrap(), ActionPlayer::Regular(PlayerId::new()), MatchActionType::Mvp, "B".to_string(), ActionId("a2".to_string()), CoachId::new());
        assert_eq!(pm3.home_actions.len(), 2);
    }

    #[test]
    fn delete_action_removes_entry() {
        let pm = make_pm(1000, 1000);
        let (pm2, _) = make_action(&pm, TeamSide::Home);
        let action_id = pm2.home_actions[0].id.clone();
        let (pm3, event) = pm2.delete_action(&action_id, CoachId::new()).unwrap();
        assert!(pm3.home_actions.is_empty());
        assert!(matches!(event, MatchReportDomainEvent::ActionDeleted { .. }));
    }

    #[test]
    fn delete_action_fails_when_not_found() {
        let pm = make_pm(1000, 1000);
        let result = pm.delete_action(&ActionId("missing".to_string()), CoachId::new());
        assert!(matches!(result, Err(DomainError::ActionNotFound(_))));
    }

    #[test]
    fn actions_for_returns_correct_side() {
        let pm = make_pm(1000, 1000);
        let (pm2, _) = make_action(&pm, TeamSide::Home);
        assert_eq!(pm2.actions_for(TeamSide::Home).len(), 1);
        assert_eq!(pm2.actions_for(TeamSide::Away).len(), 0);
    }

    #[test]
    fn star_player_uids_for_returns_engaged_uids() {
        let mut pm = make_pm(1000, 1000);
        let uid = InducementId("SP1".to_string());
        pm.star_engagements.push((pm.home_team_id.clone(), uid.clone()));
        let result = pm.star_player_uids_for(&pm.home_team_id.clone());
        assert_eq!(result, vec![uid]);
        assert!(pm.star_player_uids_for(&pm.away_team_id.clone()).is_empty());
    }

    #[test]
    fn purchases_for_returns_team_inducements() {
        let mut pm = make_pm(1000, 1000);
        pm.home_inducements = Some(vec![]);
        assert_eq!(pm.purchases_for(&pm.home_team_id.clone()).len(), 0);
        assert_eq!(pm.purchases_for(&pm.away_team_id.clone()).len(), 0);
    }
}
