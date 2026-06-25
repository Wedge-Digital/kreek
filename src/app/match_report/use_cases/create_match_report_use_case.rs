use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::MatchReportOrigin;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::common_types::{
    CoachId, CompetitionId, EventId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;
use crate::common::services::event_bus::event_bus::EventBus;

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
    app_event_bus: &EventBus,
) -> Result<MatchReportId, CreateMatchReportError> {
    if let Ok(Some(existing_id)) = repo
        .find_id_by_round_and_teams(
            &cmd.round_id.to_string(),
            &cmd.home_team_id.to_string(),
            &cmd.away_team_id.to_string(),
        )
        .await
    {
        let mr_id = MatchReportId::try_new(&existing_id)
            .map_err(|e| CreateMatchReportError::Repository(e.to_string()))?;

        return confirm_existing(mr_id, cmd.created_by, repo, app_event_bus).await;
    }

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

async fn confirm_existing(
    mr_id: MatchReportId,
    confirmed_by: CoachId,
    repo: &dyn IMatchReportRepository,
    app_event_bus: &EventBus,
) -> Result<MatchReportId, CreateMatchReportError> {
    let mr_id_str = mr_id.to_string();

    let state = repo
        .find_by_id(&mr_id_str)
        .await
        .map_err(|e| CreateMatchReportError::Repository(e.to_string()))?
        .ok_or_else(|| CreateMatchReportError::Repository("rapport introuvable".into()))?;

    match state {
        MatchReportState::Draft(draft) => {
            let space_id = draft.space_id.to_string();
            let home_id = draft.home_team_id.to_string();
            let away_id = draft.away_team_id.to_string();

            let (pre_match, confirm_event) = draft.confirm_selection(confirmed_by);

            repo.append(&mr_id_str, &confirm_event, pre_match.version - 1)
                .await
                .map_err(|e| CreateMatchReportError::Repository(e.to_string()))?;

            let _ = app_event_bus.send(
                MatchReportAppEvent::MatchReportConfirmed {
                    event_id: EventId::new(),
                    match_report_id: mr_id_str,
                    home_team_id: home_id,
                    away_team_id: away_id,
                    space_id,
                }
                .to_enveloppe(),
            );

            Ok(mr_id)
        }
        MatchReportState::PreMatch(_) => Ok(mr_id),
        MatchReportState::Cancelled(_) => {
            Err(CreateMatchReportError::Repository("rapport annulé".into()))
        }
    }
}
