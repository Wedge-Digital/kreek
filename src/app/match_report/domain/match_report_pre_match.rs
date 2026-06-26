use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::value_objects::{
    AllowedInducementSpec, D3Roll, InducementPurchase, MatchReportOrigin, TeamValue,
};
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::team::TeamId;

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
            let tv_diff = self.home_team_value.unwrap().0.abs_diff(self.away_team_value.unwrap().0);
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
            if qty > &spec.max_qty {
                return Err(DomainError::MaxQtyExceeded { uid: uid.0.clone(), qty: *qty, max_qty: spec.max_qty });
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
        .filter(|(uid, _)| allowed_specs.iter().any(|s| &s.uid == uid && s.is_star_player))
        .count();
    if star_count > 2 { Err(DomainError::StarPlayerLimitExceeded) } else { Ok(()) }
}

fn validate_star_player_conflict(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
    opponent_star_uids: &[InducementId],
) -> Result<(), DomainError> {
    for (uid, _) in purchases {
        let is_star = allowed_specs.iter().any(|s| &s.uid == uid && s.is_star_player);
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
        .filter_map(|(uid, qty)| allowed_specs.iter().find(|s| &s.uid == uid).map(|s| s.unit_cost * (*qty as u32)))
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
                qty: *qty,
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
        if allowed_specs.iter().any(|s| s.uid == p.uid && s.is_star_player) {
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
    use crate::app::match_report::domain::value_objects::MatchReportOrigin;
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
            home_team_value: Some(TeamValue(home_tv)), away_team_value: Some(TeamValue(away_tv)),
            home_inducements: None, away_inducements: None, version: 1,
        }
    }

    fn spec(uid: &str, max_qty: u8, unit_cost: u32, is_star: bool) -> AllowedInducementSpec {
        AllowedInducementSpec { uid: InducementId(uid.to_string()), max_qty, unit_cost, is_star_player: is_star }
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
}
