//! Le seul point de conversion domain event → app event du BC `teams`.
//!
//! Il s'abonne au bus interne alimenté par `TeamRepository` et republie sur le
//! bus applicatif ce qui intéresse les autres BCs. Ni use case ni handler
//! n'accède à l'`app_event_bus` : pour qu'un fait sorte de `teams`, il doit
//! d'abord être un événement domaine.

use crate::app::shared_kernel::app_events::teams_app_events::TeamsAppEvent;
use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{EventId, SpaceId};
use crate::app::teams::domain::team::TeamDomainEvent;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;

pub fn teams_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus, pool: PgPool) {
    let mut rx = event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<TeamDomainEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    if let Some(app_event) = to_app_event(&event, &envelope.emitter, &pool).await {
                        let _ = app_event_bus.send(app_event.to_enveloppe());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("teams_app_event_publisher: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// L'émetteur de l'enveloppe est le `team_id`. Le `space_id`, lui, appartient à
/// l'équipe et non au recrutement : le porter dans chaque événement domaine
/// serait du bruit, on le relit donc sur la projection au moment de sortir du
/// BC.
async fn to_app_event(
    event: &TeamDomainEvent,
    team_id: &str,
    pool: &PgPool,
) -> Option<TeamsAppEvent> {
    match event {
        TeamDomainEvent::PlayerRecruited {
            player_id,
            roster_line,
            base_value_kpo,
            ..
        } => Some(TeamsAppEvent::PlayerRecruited {
            event_id: EventId::new(),
            team_id: TeamId::try_new(team_id).ok()?,
            space_id: space_id_de(team_id, pool).await?,
            player_id: PlayerId::try_new(&player_id.to_string()).ok()?,
            roster_line_id: roster_line.0.clone(),
            base_value_kpo: base_value_kpo.0,
        }),
        _ => None,
    }
}

async fn space_id_de(team_id: &str, pool: &PgPool) -> Option<SpaceId> {
    let (space_id,): (String,) =
        sqlx::query_as("SELECT space_id FROM team_proj WHERE team_id = $1")
            .bind(team_id)
            .fetch_optional(pool)
            .await
            .ok()??;
    SpaceId::try_new(&space_id).ok()
}
