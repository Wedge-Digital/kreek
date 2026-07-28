use crate::app::shared_kernel::app_events::player_improvement_app_events::PlayerImprovementAppEvent;
use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::teams::domain::value_objects::{Kpo, PlayerImprovement, Stat};
use crate::app::teams::ports::ITeamRepository;
use crate::common::services::event_bus::event_bus::EventBus;
use std::sync::Arc;

/// Écoute les achats de compétence/caractéristique côté BC `players` pour
/// construire `TeamDomainEvent::PlayerImprovementApplied` (déjà défini, mais
/// jusqu'ici jamais construit) et refléter la valeur ajoutée sur `team_value`.
pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<PlayerImprovementAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    handle_event(&team_repo, app_event).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("player_improvement_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(team_repo: &Arc<dyn ITeamRepository>, app_event: PlayerImprovementAppEvent) {
    let (team_id, player_id, improvement, value_delta_po) = match app_event {
        PlayerImprovementAppEvent::SkillPurchased { team_id, player_id, skill_name, value_delta_po } => {
            (team_id, player_id, PlayerImprovement::NewSkill(skill_name), value_delta_po)
        }
        PlayerImprovementAppEvent::StatIncreased { team_id, player_id, stat, value_delta_po } => {
            let Some(stat) = parse_stat(&stat) else {
                tracing::warn!("player_improvement_listener: stat inconnu {stat}");
                return;
            };
            (team_id, player_id, PlayerImprovement::StatBoost(stat), value_delta_po)
        }
    };

    let team = match team_repo.find_by_id(&team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            tracing::warn!("player_improvement_listener: team {team_id} not found");
            return;
        }
        Err(e) => {
            tracing::error!("player_improvement_listener: find_by_id {team_id}: {e}");
            return;
        }
    };

    let Ok(player_id) = PlayerId::try_new(&player_id) else {
        tracing::warn!("player_improvement_listener: player_id invalide {player_id}");
        return;
    };

    // value_delta_po est en Po (players::ValueKpo) — teams::Kpo stocke déjà des
    // kPo, d'où la division par 1000 (cf. shared_kernel::player_improvement_app_events).
    let value_delta = Kpo(value_delta_po / 1000);
    let event = team.apply_player_improvement(player_id, improvement, value_delta);

    if let Err(e) = team_repo.append(&team_id, &event, team.version).await {
        tracing::error!("player_improvement_listener: append {team_id}: {e}");
    }
}

fn parse_stat(raw: &str) -> Option<Stat> {
    match raw {
        "Ma" => Some(Stat::Ma),
        "St" => Some(Stat::St),
        "Ag" => Some(Stat::Ag),
        "Pa" => Some(Stat::Pa),
        "Av" => Some(Stat::Av),
        _ => None,
    }
}
