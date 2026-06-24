use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::value_objects::MatchReportOrigin;
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;

pub struct CreateMatchReportCommand {
    pub space_id: SpaceId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub created_by: CoachId,
    pub origin: MatchReportOrigin,
    pub pairing_id: Option<String>,
}

#[derive(Debug)]
pub enum CreateMatchReportError {
    SameTeam,
    Repository(String),
}

pub async fn execute(
    cmd: CreateMatchReportCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<MatchReportId, CreateMatchReportError> {
    let id = MatchReportId::new();

    let (_draft, event) = MatchReportDraft::create(
        id,
        cmd.space_id,
        cmd.competition_id,
        cmd.season_id,
        cmd.round_id,
        cmd.home_team_id,
        cmd.away_team_id,
        cmd.created_by,
        cmd.origin,
        cmd.pairing_id,
    )
    .map_err(|_| CreateMatchReportError::SameTeam)?;

    repo.append(&id.to_string(), &event, 0)
        .await
        .map_err(|e| CreateMatchReportError::Repository(e.to_string()))?;

    Ok(id)
}
