use std::sync::Arc;
use crate::app::shared_kernel::app_events::team_creation_app_events::TeamCreationAppEvent;
use crate::app::teams::domain::team::TeamDomainEvent;
use crate::app::teams::domain::value_objects::Kpo;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) = serde_json::from_value::<TeamCreationAppEvent>(envelope.payload.clone()) else {
                        continue;
                    };
                    if let TeamCreationAppEvent::TeamCreated {
                        team_id, space_id, team_name,
                        roster_id, roster_name,
                        coach_id, coach_name, treasury, ..
                    } = app_event {
                        let domain_event = TeamDomainEvent::TeamCreated {
                            team_id:     team_id.clone(),
                            space_id,
                            name:        team_name,
                            roster_id,
                            roster_name,
                            coach_id,
                            coach_name,
                            treasury:    Kpo(treasury),
                        };
                        if let Err(e) = team_repo.append(&team_id, &domain_event, 0).await {
                            match e {
                                RepositoryError::ConcurrentWrite =>
                                    tracing::warn!("teams team_created_listener: TeamCreated déjà persisté pour {team_id}"),
                                other =>
                                    tracing::error!("teams team_created_listener: échec append pour {team_id}: {other}"),
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("teams team_created_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
