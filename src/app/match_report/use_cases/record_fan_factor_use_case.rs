use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{D3Roll, TeamValue};
use crate::app::match_report::ports::{ICompetitionDataPort, ITeamDataPort};
use crate::app::shared_kernel::common_types::{CoachId, MatchReportId};

pub struct RecordFanFactorCommand {
    pub match_report_id: MatchReportId,
    pub home_fan_roll: D3Roll,
    pub away_fan_roll: D3Roll,
    pub recorded_by: CoachId,
}

#[derive(Debug)]
pub enum RecordFanFactorOutcome {
    RedirectToInducements { topdog_team_id: String },
    RedirectToStep3,
}

#[derive(Debug)]
pub enum RecordFanFactorError {
    NotFound,
    NotInPreMatchPhase,
    TeamValueUnavailable(String),
    Repository(String),
}

pub async fn execute(
    cmd: RecordFanFactorCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> Result<RecordFanFactorOutcome, RecordFanFactorError> {
    let mr_id = cmd.match_report_id.to_string();
    let pre_match = load_pre_match(repo, &mr_id).await?;
    let (updated, events) = build_events(pre_match, cmd, team_data).await?;
    let version_before = updated.version - events.len() as u64;
    repo.append_many(&mr_id, events, version_before)
        .await
        .map_err(|e| RecordFanFactorError::Repository(e.to_string()))?;
    determine_outcome(&updated, team_data, competition_data).await
}

async fn load_pre_match(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportPreMatch, RecordFanFactorError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(|e| RecordFanFactorError::Repository(e.to_string()))?
        .ok_or(RecordFanFactorError::NotFound)?;
    match state {
        MatchReportState::PreMatch(pm) => Ok(pm),
        _ => Err(RecordFanFactorError::NotInPreMatchPhase),
    }
}

async fn build_events(
    pm: MatchReportPreMatch,
    cmd: RecordFanFactorCommand,
    team_data: &dyn ITeamDataPort,
) -> Result<
    (
        MatchReportPreMatch,
        Vec<crate::app::match_report::domain::events::MatchReportDomainEvent>,
    ),
    RecordFanFactorError,
> {
    let (updated_ff, ff_event) =
        pm.record_fan_factor(cmd.home_fan_roll, cmd.away_fan_roll, cmd.recorded_by.clone());
    let (home_tv, away_tv) = fetch_team_values(&updated_ff, team_data).await?;
    let (updated_tv, tv_event) =
        updated_ff.record_team_values(home_tv, away_tv, cmd.recorded_by);
    Ok((updated_tv, vec![ff_event, tv_event]))
}

async fn fetch_team_values(
    pm: &MatchReportPreMatch,
    team_data: &dyn ITeamDataPort,
) -> Result<(TeamValue, TeamValue), RecordFanFactorError> {
    let home_id = pm.home_team_id.to_string();
    let away_id = pm.away_team_id.to_string();
    let (home_raw, away_raw) =
        tokio::join!(team_data.find_team_value(&home_id), team_data.find_team_value(&away_id));
    let home_tv = home_raw
        .map(TeamValue)
        .ok_or_else(|| RecordFanFactorError::TeamValueUnavailable(home_id))?;
    let away_tv = away_raw
        .map(TeamValue)
        .ok_or_else(|| RecordFanFactorError::TeamValueUnavailable(away_id))?;
    Ok((home_tv, away_tv))
}

async fn determine_outcome(
    pm: &MatchReportPreMatch,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> Result<RecordFanFactorOutcome, RecordFanFactorError> {
    let topdog_id = pm.topdog_team_id().to_string();
    let roster_id = team_data
        .find_team_info(&topdog_id)
        .await
        .map(|info| info.roster_id)
        .unwrap_or_default();
    let tier_rules = competition_data
        .find_tier_rules_for_roster(&pm.season_id.to_string(), &roster_id)
        .await;
    let has_inducements = tier_rules
        .as_ref()
        .map(|r| !r.allowed_inducements.is_empty() || !r.allowed_star_players.is_empty())
        .unwrap_or(false);
    if has_inducements {
        Ok(RecordFanFactorOutcome::RedirectToInducements { topdog_team_id: topdog_id })
    } else {
        Ok(RecordFanFactorOutcome::RedirectToStep3)
    }
}
