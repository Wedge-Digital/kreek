use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{D3Roll, DedicatedFans, TeamValue};
use crate::app::match_report::ports::{ICompetitionDataPort, ITeamDataPort};
use crate::app::match_report::use_cases::conflict_retry::ConflictAware;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;

#[derive(Debug)]
pub struct RecordFanFactorCommand {
    pub match_report_id: MatchReportId,
    pub home_fan_roll: D3Roll,
    pub away_fan_roll: D3Roll,
    pub home_dedicated_fans: DedicatedFans,
    pub away_dedicated_fans: DedicatedFans,
    pub recorded_by: CoachId,
}

#[derive(Debug)]
pub enum RecordFanFactorOutcome {
    /// La page de coups de pouce de cette équipe : le favori en « Je saisis
    /// tout », l'équipe du coach en « Chacun son équipe » (carte 654).
    RedirectToInducements { team_id: String },
    /// Pas de coups de pouce : le coup d'envoi, au match en direct (carte 636).
    RedirectToLiveMatch,
}

#[derive(Debug)]
pub enum RecordFanFactorError {
    NotFound,
    NotInPreMatchPhase,
    TeamValueUnavailable(String),
    /// Le domaine refuse : en « Chacun son équipe », les deux jets d'un coup
    /// écraseraient celui de l'autre coach (carte 654).
    Domain(DomainError),
    /// Une autre écriture est passée entre le chargement et l'append (carte 623).
    Conflict,
    Repository(String),
}

impl ConflictAware for RecordFanFactorError {
    fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

impl RecordFanFactorError {
    /// `ConcurrentWrite` garde son sens au lieu de se fondre dans `Repository`.
    pub(crate) fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Repository(other.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    mut cmd: RecordFanFactorCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> Result<RecordFanFactorOutcome, RecordFanFactorError> {
    let mr_id = cmd.match_report_id.to_string();
    let pre_match = load_pre_match(repo, &mr_id).await?;
    let (home_fans, away_fans) = fetch_dedicated_fans(&pre_match, team_data).await;
    cmd.home_dedicated_fans = DedicatedFans::try_new(home_fans).unwrap_or_default();
    cmd.away_dedicated_fans = DedicatedFans::try_new(away_fans).unwrap_or_default();
    let (updated, events) = build_events(pre_match, cmd, team_data).await?;
    let version_before = updated.version - events.len() as u64;
    repo.append_many(&mr_id, events, version_before)
        .await
        .map_err(RecordFanFactorError::from_repository)?;
    determine_outcome(&updated, team_data, competition_data).await
}

async fn fetch_dedicated_fans(
    pm: &MatchReportPreMatch,
    team_data: &dyn ITeamDataPort,
) -> (u32, u32) {
    let home_id = pm.home_team_id.to_string();
    let away_id = pm.away_team_id.to_string();
    let (home_info, away_info) = tokio::join!(
        team_data.find_team_info(&home_id),
        team_data.find_team_info(&away_id),
    );
    let home = home_info.map(|i| i.dedicated_fans).unwrap_or(0);
    let away = away_info.map(|i| i.dedicated_fans).unwrap_or(0);
    (home, away)
}

pub(crate) async fn load_pre_match(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportPreMatch, RecordFanFactorError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(RecordFanFactorError::from_repository)?
        .ok_or(RecordFanFactorError::NotFound)?;
    match state {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
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
    let (updated_ff, ff_event) = pm
        .record_fan_factor(
            cmd.home_fan_roll,
            cmd.away_fan_roll,
            cmd.home_dedicated_fans,
            cmd.away_dedicated_fans,
            cmd.recorded_by.clone(),
        )
        .map_err(RecordFanFactorError::Domain)?;
    let (home_tv, away_tv) = fetch_team_values(&updated_ff, team_data).await?;
    let (updated_tv, tv_event) = updated_ff.record_team_values(home_tv, away_tv, cmd.recorded_by);
    Ok((updated_tv, vec![ff_event, tv_event]))
}

pub(crate) async fn fetch_team_values(
    pm: &MatchReportPreMatch,
    team_data: &dyn ITeamDataPort,
) -> Result<(TeamValue, TeamValue), RecordFanFactorError> {
    let home_id = pm.home_team_id.to_string();
    let away_id = pm.away_team_id.to_string();
    let (home_raw, away_raw) = tokio::join!(
        team_data.find_team_value(&home_id),
        team_data.find_team_value(&away_id)
    );
    let home_tv = home_raw
        .and_then(|v| TeamValue::try_new(v).ok())
        .ok_or_else(|| RecordFanFactorError::TeamValueUnavailable(home_id))?;
    let away_tv = away_raw
        .and_then(|v| TeamValue::try_new(v).ok())
        .ok_or_else(|| RecordFanFactorError::TeamValueUnavailable(away_id))?;
    Ok((home_tv, away_tv))
}

async fn determine_outcome(
    pm: &MatchReportPreMatch,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> Result<RecordFanFactorOutcome, RecordFanFactorError> {
    let topdog_id = pm.topdog_team_id().to_string();
    Ok(next_step(pm, topdog_id, team_data, competition_data).await)
}

/// La suite du fan factor : les coups de pouce de `team_id` si le palier en
/// a, sinon le match. Partagée avec le fan factor d'un camp (carte 654).
pub(crate) async fn next_step(
    pm: &MatchReportPreMatch,
    team_id: String,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> RecordFanFactorOutcome {
    let roster_id = team_data
        .find_team_info(&team_id)
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
        RecordFanFactorOutcome::RedirectToInducements { team_id }
    } else {
        RecordFanFactorOutcome::RedirectToLiveMatch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, ConflictingRepository, PlainCompetition, ReadyTeams,
    };

    #[tokio::test]
    async fn un_conflit_d_ecriture_est_relaye() {
        let repo = ConflictingRepository::holding(MatchReportState::PreMatch(pre_match()));
        let cmd = RecordFanFactorCommand {
            match_report_id: MatchReportId::new(),
            home_fan_roll: D3Roll::try_new(2).unwrap(),
            away_fan_roll: D3Roll::try_new(2).unwrap(),
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            recorded_by: CoachId::new(),
        };
        let resultat = execute(cmd, &repo, &ReadyTeams, &PlainCompetition).await;
        assert!(
            matches!(resultat, Err(RecordFanFactorError::Conflict)),
            "{resultat:?}"
        );
    }
}
