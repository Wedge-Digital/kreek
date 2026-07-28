use crate::app::shared_kernel::app_events::match_report_app_events::{
    MatchReportAppEvent, MatchReportUnpublishedPayload,
};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::teams::domain::team::Team;
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Défait la séquence d'après-match sur les 2 équipes d'un rapport dépublié :
/// trésorerie, fans et phase de jeu retrouvent leur état d'avant publication.
pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(MatchReportAppEvent::MatchReportUnpublished(payload)) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    handle_unpublished(&payload, team_repo.as_ref()).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("teams::match_report_unpublished_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_unpublished(payload: &MatchReportUnpublishedPayload, repo: &dyn ITeamRepository) {
    let Ok(mr_id) = MatchReportId::try_new(&payload.match_report_id) else {
        tracing::warn!(
            "teams::match_report_unpublished_listener: id invalide {}",
            payload.match_report_id
        );
        return;
    };

    for team_id in [&payload.home_team_id, &payload.away_team_id] {
        revert_team(repo, team_id, mr_id).await;
    }
}

/// Chaque équipe est traitée indépendamment : l'échec de l'une ne doit pas
/// priver l'autre de sa compensation.
async fn revert_team(repo: &dyn ITeamRepository, team_id: &str, mr_id: MatchReportId) {
    let Some(team) = load_team(repo, team_id).await else {
        return;
    };

    // Un refus du domaine n'est pas une anomalie : c'est aussi ce qui rend la
    // compensation idempotente quand elle est rejouée.
    let event = match team.revert_post_match_sequence(mr_id) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                "teams::match_report_unpublished_listener: revert refusé pour {team_id}: {e}"
            );
            return;
        }
    };

    if let Err(e) = repo.append(team_id, &event, team.version).await {
        tracing::error!("teams::match_report_unpublished_listener: append {team_id}: {e}");
    }
}

async fn load_team(repo: &dyn ITeamRepository, team_id: &str) -> Option<Team> {
    match repo.find_by_id(team_id).await {
        Ok(Some(t)) => Some(t),
        Ok(None) => {
            tracing::warn!("teams::match_report_unpublished_listener: équipe {team_id} introuvable");
            None
        }
        Err(e) => {
            tracing::error!("teams::match_report_unpublished_listener: find_by_id {team_id}: {e}");
            None
        }
    }
}
