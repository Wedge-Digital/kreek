use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};

pub struct DashboardSummary {
    pub enrolled_count: usize,
    pub pending_count: usize,
    pub matches_played: usize,
    pub matches_total: usize,
    pub rounds_validated: usize,
    pub rounds_total: usize,
    pub max_participants: Option<u32>,
    pub recent_activity: Vec<ActivityEntry>,
}

pub struct ActivityEntry {
    pub kind: ActivityKind,
    pub description: String,
    pub occurred_at: String,
}

pub enum ActivityKind {
    Enrollment,
    MatchResult,
    Validation,
    RestDay,
}

#[derive(Debug)]
pub enum QueryError {
    CompetitionNotFound,
    SeasonNotFound,
    Repository(String),
}

// arch:no-instrument — lecture : répond à « qu'affiche-t-on », pas à « qu'a-t-on demandé »
pub async fn execute(
    competition_id: &CompetitionId,
    season_id: &SeasonId,
    competition_repo: &dyn ICompetitionRepository,
    season_repo: &dyn ISeasonRepository,
) -> Result<DashboardSummary, QueryError> {
    let _comp = competition_repo
        .find_base_info(competition_id)
        .await
        .map_err(|e| QueryError::Repository(e.to_string()))?
        .ok_or(QueryError::CompetitionNotFound)?;

    let invitations = season_repo
        .find_invitations(season_id)
        .await
        .map_err(|e| QueryError::Repository(e.to_string()))?;

    let max_participants = invitations.and_then(|inv| inv.max_participants);

    Ok(DashboardSummary {
        enrolled_count: 0,
        pending_count: 0,
        matches_played: 0,
        matches_total: 0,
        rounds_validated: 0,
        rounds_total: 0,
        max_participants,
        recent_activity: vec![],
    })
}
