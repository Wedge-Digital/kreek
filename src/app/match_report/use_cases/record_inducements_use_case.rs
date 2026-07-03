use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{AllowedInducementSpec, IsStarPlayer, RosterPositionUid, TeamValue};
use crate::app::match_report::ports::{
    ICompetitionDataPort, IPlayerDataPort, ITeamDataPort, PositionCountDto, RosterPositionDto,
    TierRulesDto,
};
use crate::app::match_report::use_cases::init_temp_players_use_case::{self, InitTempPlayersCommand};
use crate::app::shared_kernel::common_types::{CoachId, MatchReportId};
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::team::TeamId;
use std::collections::HashMap;

// ── Commande ──────────────────────────────────────────────────────────────────

pub struct InducementPurchaseCmd {
    pub uid: InducementId,
    pub qty: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MercenaryLevel {
    Base,
    Lvl1,
}

impl MercenaryLevel {
    pub fn extra_cost(&self) -> u32 {
        match self { Self::Base => 30, Self::Lvl1 => 80 }
    }

    pub fn as_str(&self) -> &'static str {
        match self { Self::Base => "base", Self::Lvl1 => "lvl1" }
    }

    pub fn try_from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "base" => Ok(Self::Base),
            "lvl1" => Ok(Self::Lvl1),
            _ => Err("niveau inconnu"),
        }
    }
}

pub struct MercenaryPurchaseCmd {
    pub position_id: RosterPositionUid,
    pub level:       MercenaryLevel,
}

pub struct RecordInducementsCommand {
    pub match_report_id:     MatchReportId,
    pub team_id:             TeamId,
    pub purchases:           Vec<InducementPurchaseCmd>,
    pub mercenary_purchases: Vec<MercenaryPurchaseCmd>,
    pub recorded_by:         CoachId,
}

// ── Résultats ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum RecordInducementsOutcome {
    RedirectToInducements { next_team_id: String },
    RedirectToStep3,
}

#[derive(Debug)]
pub enum RecordInducementsError {
    NotFound,
    NotInPreMatchPhase,
    TeamValuesNotRecorded,
    TreasuryUnavailable(String),
    TierRulesUnavailable(String),
    UnauthorizedInducement(String),
    Domain(DomainError),
    Repository(String),
    InvalidMercenaryPosition(RosterPositionUid),
    PlayerCountUnavailable(String),
}

// ── Execute ───────────────────────────────────────────────────────────────────

pub async fn execute(
    cmd: RecordInducementsCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
    player_data: &dyn IPlayerDataPort,
) -> Result<RecordInducementsOutcome, RecordInducementsError> {
    let mr_id = cmd.match_report_id.to_string();
    let mut pm = load_pre_match(repo, &mr_id).await?;
    if pm.home_team_value.is_none() || pm.away_team_value.is_none() {
        let home_id = pm.home_team_id.to_string();
        let away_id = pm.away_team_id.to_string();
        let (home_raw, away_raw) = tokio::join!(
            team_data.find_team_value(&home_id),
            team_data.find_team_value(&away_id),
        );
        pm.home_team_value = home_raw.and_then(|v| TeamValue::try_new(v).ok());
        pm.away_team_value = away_raw.and_then(|v| TeamValue::try_new(v).ok());
        if pm.home_team_value.is_none() || pm.away_team_value.is_none() {
            return Err(RecordInducementsError::TeamValuesNotRecorded);
        }
    }
    let tier = fetch_tier_rules(&pm, &cmd.team_id, team_data, competition_data).await?;
    validate_purchase_uids(&cmd.purchases, &tier)?;
    let roster_positions = team_data.find_roster_positions(&cmd.team_id.to_string()).await;
    let validated_mercs = validate_mercenary_positions(&cmd.mercenary_purchases, &roster_positions)?;
    let player_counts = fetch_player_counts(&cmd.team_id, player_data).await?;
    validate_roster_availability(&validated_mercs, &player_counts)?;
    let treasury = fetch_treasury(&cmd.team_id, team_data).await?;
    let budget = pm.inducement_budget_for(&cmd.team_id, treasury);
    let (allowed_specs, purchases_tuples) =
        build_all_specs_and_purchases(&tier, &cmd.purchases, &validated_mercs);
    let opponent_star_uids = collect_opponent_star_uids(&pm, &cmd.team_id);
    let (updated_pm, events) = pm
        .record_inducements(&cmd.team_id, &purchases_tuples, budget, &allowed_specs, &opponent_star_uids, cmd.recorded_by)
        .map_err(RecordInducementsError::Domain)?;
    persist_and_init(&mr_id, events, updated_pm.version, cmd.match_report_id, cmd.team_id, repo, team_data, player_data).await?;
    Ok(route_outcome(&updated_pm, &cmd.team_id))
}

// ── Validation helpers ────────────────────────────────────────────────────────

async fn load_pre_match(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportPreMatch, RecordInducementsError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(|e| RecordInducementsError::Repository(e.to_string()))?
        .ok_or(RecordInducementsError::NotFound)?;
    match state {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
        _ => Err(RecordInducementsError::NotInPreMatchPhase),
    }
}

async fn fetch_tier_rules(
    pm: &MatchReportPreMatch,
    team_id: &TeamId,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> Result<TierRulesDto, RecordInducementsError> {
    let roster_id = team_data
        .find_team_info(&team_id.to_string())
        .await
        .map(|i| i.roster_id)
        .unwrap_or_default();
    competition_data
        .find_tier_rules_for_roster(&pm.season_id.to_string(), &roster_id)
        .await
        .ok_or_else(|| RecordInducementsError::TierRulesUnavailable(team_id.to_string()))
}

fn validate_purchase_uids(
    purchases: &[InducementPurchaseCmd],
    tier: &TierRulesDto,
) -> Result<(), RecordInducementsError> {
    let all_allowed: Vec<&str> = tier
        .allowed_inducements
        .iter()
        .chain(&tier.allowed_star_players)
        .map(|s| s.uid.as_str())
        .collect();
    for p in purchases {
        if !all_allowed.contains(&p.uid.0.as_str()) {
            return Err(RecordInducementsError::UnauthorizedInducement(p.uid.0.clone()));
        }
    }
    Ok(())
}

fn validate_mercenary_positions(
    purchases: &[MercenaryPurchaseCmd],
    roster_positions: &[RosterPositionDto],
) -> Result<Vec<ValidatedMercenary>, RecordInducementsError> {
    purchases.iter().map(|cmd| {
        let pos = roster_positions
            .iter()
            .find(|p| p.position_uid == cmd.position_id.to_string())
            .ok_or_else(|| RecordInducementsError::InvalidMercenaryPosition(cmd.position_id.clone()))?;
        Ok(ValidatedMercenary {
            position_id: cmd.position_id.clone(),
            level:       cmd.level.clone(),
            cost:        pos.base_cost + cmd.level.extra_cost(),
            max_qty:     pos.max_qty,
        })
    }).collect()
}

async fn fetch_player_counts(
    team_id: &TeamId,
    player_data: &dyn IPlayerDataPort,
) -> Result<Vec<PositionCountDto>, RecordInducementsError> {
    Ok(player_data.find_player_counts_by_position(&team_id.to_string()).await)
}

fn validate_roster_availability(
    mercs: &[ValidatedMercenary],
    player_counts: &[PositionCountDto],
) -> Result<(), RecordInducementsError> {
    let mut req_counts: HashMap<String, u8> = HashMap::new();
    for m in mercs {
        *req_counts.entry(m.position_id.to_string()).or_insert(0) += 1;
    }
    for (pos_id, req_count) in &req_counts {
        let count_in_team = player_counts.iter().find(|c| &c.position_uid == pos_id).map(|c| c.count).unwrap_or(0);
        let max_qty = mercs.iter().find(|m| m.position_id.to_string() == *pos_id).map(|m| m.max_qty).unwrap_or(0);
        if count_in_team + req_count > max_qty {
            return Err(RecordInducementsError::Domain(DomainError::MaxQtyExceeded {
                uid: format!("MERCO:{pos_id}:*"), qty: count_in_team + req_count, max_qty,
            }));
        }
    }
    Ok(())
}

async fn fetch_treasury(
    team_id: &TeamId,
    team_data: &dyn ITeamDataPort,
) -> Result<u32, RecordInducementsError> {
    team_data
        .find_team_treasury(&team_id.to_string())
        .await
        .ok_or_else(|| RecordInducementsError::TreasuryUnavailable(team_id.to_string()))
}

// ── Build specs + purchases ───────────────────────────────────────────────────

struct ValidatedMercenary {
    position_id: RosterPositionUid,
    level:       MercenaryLevel,
    cost:        u32,
    max_qty:     u8,
}

fn build_all_specs_and_purchases(
    tier: &TierRulesDto,
    purchases: &[InducementPurchaseCmd],
    validated_mercs: &[ValidatedMercenary],
) -> (Vec<AllowedInducementSpec>, Vec<(InducementId, u8)>) {
    let mut specs = build_allowed_specs(tier);
    let mut tuples: Vec<(InducementId, u8)> = purchases.iter().map(|p| (p.uid.clone(), p.qty)).collect();
    let (merco_specs, merco_tuples) = build_merco_specs(validated_mercs);
    specs.extend(merco_specs);
    tuples.extend(merco_tuples);
    (specs, tuples)
}

fn build_allowed_specs(tier: &TierRulesDto) -> Vec<AllowedInducementSpec> {
    use crate::app::match_report::domain::value_objects::{InducementCost, InducementQty};
    let inducements = tier.allowed_inducements.iter().filter_map(|s| {
        Some(AllowedInducementSpec {
            uid: InducementId(s.uid.clone()),
            max_qty: InducementQty::try_new(s.max_qty).ok()?,
            unit_cost: InducementCost::try_new(s.unit_cost).ok()?,
            is_star_player: IsStarPlayer(false),
        })
    });
    let stars = tier.allowed_star_players.iter().filter_map(|s| {
        Some(AllowedInducementSpec {
            uid: InducementId(s.uid.clone()),
            max_qty: InducementQty::try_new(s.max_qty).ok()?,
            unit_cost: InducementCost::try_new(s.unit_cost).ok()?,
            is_star_player: IsStarPlayer(true),
        })
    });
    inducements.chain(stars).collect()
}

fn build_merco_specs(
    mercs: &[ValidatedMercenary],
) -> (Vec<AllowedInducementSpec>, Vec<(InducementId, u8)>) {
    use crate::app::match_report::domain::value_objects::{InducementCost, InducementQty};
    let mut groups: HashMap<String, (u32, u8, u8)> = HashMap::new();
    for merc in mercs {
        let uid = format!("MERCO:{}:{}", merc.position_id, merc.level.as_str());
        let entry = groups.entry(uid).or_insert((merc.cost, 0, merc.max_qty));
        entry.1 += 1;
    }
    let mut specs = vec![];
    let mut tuples = vec![];
    for (uid, (cost, qty, max_qty_for_pos)) in groups {
        let induction_id = InducementId(uid);
        if let (Ok(mq), Ok(uc)) = (InducementQty::try_new(max_qty_for_pos), InducementCost::try_new(cost)) {
            specs.push(AllowedInducementSpec { uid: induction_id.clone(), max_qty: mq, unit_cost: uc, is_star_player: IsStarPlayer(false) });
        }
        tuples.push((induction_id, qty));
    }
    (specs, tuples)
}

// ── Persistence ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn persist_and_init(
    mr_id: &str,
    events: Vec<crate::app::match_report::domain::events::MatchReportDomainEvent>,
    version_after: u64,
    match_report_id: MatchReportId,
    team_id: TeamId,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
) -> Result<(), RecordInducementsError> {
    let version_before = version_after - events.len() as u64;
    repo.append_many(mr_id, events, version_before)
        .await
        .map_err(|e| RecordInducementsError::Repository(e.to_string()))?;
    init_temp_players_use_case::execute(
        InitTempPlayersCommand { match_report_id, team_id },
        repo,
        team_data,
        player_data,
    )
    .await
    .map_err(|e| RecordInducementsError::Repository(format!("{e:?}")))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn collect_opponent_star_uids(pm: &MatchReportPreMatch, team_id: &TeamId) -> Vec<InducementId> {
    let opponent_inducements = if team_id == &pm.home_team_id {
        &pm.away_inducements
    } else {
        &pm.home_inducements
    };
    opponent_inducements
        .as_ref()
        .map(|purchases| purchases.iter().map(|p| p.uid.clone()).collect())
        .unwrap_or_default()
}

fn route_outcome(pm: &MatchReportPreMatch, team_id: &TeamId) -> RecordInducementsOutcome {
    if pm.topdog_team_id() == team_id {
        RecordInducementsOutcome::RedirectToInducements {
            next_team_id: pm.underdog_team_id().to_string(),
        }
    } else {
        RecordInducementsOutcome::RedirectToStep3
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::MatchReportOrigin;
    use crate::app::shared_kernel::common_types::{CompetitionId, RoundId, SeasonId, SpaceId};

    fn make_pm(home_tv: u32, away_tv: u32) -> MatchReportPreMatch {
        MatchReportPreMatch {
            id: MatchReportId::new(), space_id: SpaceId::new(),
            competition_id: CompetitionId::new(), season_id: SeasonId::new(),
            round_id: RoundId::new(), home_team_id: TeamId::new(), away_team_id: TeamId::new(),
            created_by: CoachId::new(), origin: MatchReportOrigin::Manual, pairing_id: None,
            home_fan_roll: None, away_fan_roll: None,
            home_dedicated_fans: 0, away_dedicated_fans: 0,
            home_team_value: Some(TeamValue::try_new(home_tv).unwrap()),
            away_team_value: Some(TeamValue::try_new(away_tv).unwrap()),
            home_inducements: None, away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![], away_temp_players: vec![],
            home_actions: vec![], away_actions: vec![],
            version: 1,
        }
    }

    #[test]
    fn route_outcome_sends_topdog_to_underdog_inducements() {
        let pm = make_pm(1000, 900);
        match route_outcome(&pm, pm.topdog_team_id()) {
            RecordInducementsOutcome::RedirectToInducements { next_team_id } => {
                assert_eq!(next_team_id, pm.underdog_team_id().to_string());
            }
            other => panic!("expected RedirectToInducements, got {other:?}"),
        }
    }

    #[test]
    fn route_outcome_sends_underdog_to_step3() {
        let pm = make_pm(1000, 900);
        assert!(matches!(route_outcome(&pm, pm.underdog_team_id()), RecordInducementsOutcome::RedirectToStep3));
    }

    #[test]
    fn route_outcome_ignores_already_recorded_inducements() {
        // Même si les deux équipes ont déjà un panier enregistré, la navigation
        // dépend uniquement du rôle de l'équipe qui vient de soumettre — jamais
        // de l'état persisté des paniers.
        let mut pm = make_pm(1000, 900);
        pm.home_inducements = Some(vec![]);
        pm.away_inducements = Some(vec![]);
        match route_outcome(&pm, pm.topdog_team_id()) {
            RecordInducementsOutcome::RedirectToInducements { next_team_id } => {
                assert_eq!(next_team_id, pm.underdog_team_id().to_string());
            }
            other => panic!("expected RedirectToInducements even with both carts recorded, got {other:?}"),
        }
    }
}
