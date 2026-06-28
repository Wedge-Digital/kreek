use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{AllowedInducementSpec, IsStarPlayer};
use crate::app::match_report::ports::{ICompetitionDataPort, IPlayerDataPort, ITeamDataPort, TierRulesDto};
use crate::app::match_report::use_cases::init_temp_players_use_case::{self, InitTempPlayersCommand};
use crate::app::shared_kernel::common_types::{CoachId, MatchReportId};
use crate::app::shared_kernel::inducement_definition::InducementId;
use crate::app::shared_kernel::team::TeamId;

pub struct InducementPurchaseCmd {
    pub uid: InducementId,
    pub qty: u8,
}

pub struct RecordInducementsCommand {
    pub match_report_id: MatchReportId,
    pub team_id: TeamId,
    pub purchases: Vec<InducementPurchaseCmd>,
    pub recorded_by: CoachId,
}

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
}

pub async fn execute(
    cmd: RecordInducementsCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
    player_data: &dyn IPlayerDataPort,
) -> Result<RecordInducementsOutcome, RecordInducementsError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pre_match_with_tv(repo, &mr_id).await?;
    let tier = fetch_tier_rules(&pm, &cmd.team_id, team_data, competition_data).await?;
    validate_purchase_uids(&cmd.purchases, &tier)?;
    let treasury = fetch_treasury(&cmd.team_id, team_data).await?;
    let budget = pm.inducement_budget_for(&cmd.team_id, treasury);
    let allowed_specs = build_allowed_specs(&tier);
    let opponent_star_uids = collect_opponent_star_uids(&pm, &cmd.team_id);
    let purchases_tuples: Vec<(InducementId, u8)> =
        cmd.purchases.iter().map(|p| (p.uid.clone(), p.qty)).collect();
    let (updated_pm, events) = pm
        .record_inducements(
            &cmd.team_id,
            &purchases_tuples,
            budget,
            &allowed_specs,
            &opponent_star_uids,
            cmd.recorded_by,
        )
        .map_err(RecordInducementsError::Domain)?;
    let version_before = updated_pm.version - events.len() as u64;
    repo.append_many(&mr_id, events, version_before)
        .await
        .map_err(|e| RecordInducementsError::Repository(e.to_string()))?;
    init_temp_players_use_case::execute(
        InitTempPlayersCommand { match_report_id: cmd.match_report_id, team_id: cmd.team_id },
        repo,
        team_data,
        player_data,
    )
    .await
    .map_err(|e| RecordInducementsError::Repository(format!("{e:?}")))?;
    route_outcome(&updated_pm)
}

async fn load_pre_match_with_tv(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportPreMatch, RecordInducementsError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(|e| RecordInducementsError::Repository(e.to_string()))?
        .ok_or(RecordInducementsError::NotFound)?;
    let pm = match state {
        MatchReportState::PreMatch(pm) => pm,
        _ => return Err(RecordInducementsError::NotInPreMatchPhase),
    };
    if pm.home_team_value.is_none() || pm.away_team_value.is_none() {
        return Err(RecordInducementsError::TeamValuesNotRecorded);
    }
    Ok(pm)
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

async fn fetch_treasury(
    team_id: &TeamId,
    team_data: &dyn ITeamDataPort,
) -> Result<u32, RecordInducementsError> {
    team_data
        .find_team_treasury(&team_id.to_string())
        .await
        .ok_or_else(|| RecordInducementsError::TreasuryUnavailable(team_id.to_string()))
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

fn route_outcome(pm: &MatchReportPreMatch) -> Result<RecordInducementsOutcome, RecordInducementsError> {
    if pm.is_inducements_phase_complete() {
        Ok(RecordInducementsOutcome::RedirectToStep3)
    } else {
        Ok(RecordInducementsOutcome::RedirectToInducements {
            next_team_id: pm.underdog_team_id().to_string(),
        })
    }
}
