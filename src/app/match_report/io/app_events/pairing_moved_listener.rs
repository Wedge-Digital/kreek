//! Le rapport suit son appariement quand celui-ci change de journée (carte
//! 557).
//!
//! Même forme que `pairing_deleted_listener` : l'événement de `competitions`
//! arrive par l'app event bus, le rapport est retrouvé par son appariement —
//! fiable depuis la migration de la carte 556 — et la suite est un use case
//! ordinaire, qui émet sur le bus **interne** pour que le publisher fasse
//! suivre `ranking` et `players`.

use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::use_cases::reassign_round_use_case::{self, ReassignRoundCommand};
use crate::app::shared_kernel::app_events::competitions_app_events::CompetitionsAppEvent;
use crate::app::shared_kernel::bloodbowl::ids::{MatchReportId, RoundId};
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use std::sync::Arc;
use tracing::Instrument;

pub fn init(app_event_bus: &EventBus, event_bus: &EventBus, repo: Arc<dyn IMatchReportRepository>) {
    let bus = event_bus.clone();
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<CompetitionsAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let CompetitionsAppEvent::PairingMoved {
                        pairing_id,
                        to_round_id,
                        ..
                    } = app_event
                    else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    suivre(&pairing_id, &to_round_id, repo.as_ref(), &bus)
                        .instrument(span)
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("pairing_moved_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn suivre(
    pairing_id: &str,
    to_round_id: &str,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) {
    let mr_id = match repo.find_id_by_pairing(pairing_id).await {
        Ok(Some(id)) => id,
        // Un appariement sans rapport vivant : rien à suivre.
        Ok(None) => return,
        Err(e) => {
            tracing::error!("pairing_moved_listener: find_id_by_pairing {pairing_id}: {e}");
            return;
        }
    };
    let (Ok(match_report_id), Ok(round_id)) = (
        MatchReportId::try_new(&mr_id),
        RoundId::try_new(to_round_id),
    ) else {
        tracing::error!("pairing_moved_listener: identifiant invalide ({mr_id}, {to_round_id})");
        return;
    };
    let cmd = ReassignRoundCommand {
        match_report_id,
        round_id,
    };
    match reassign_round_use_case::execute(cmd, repo, bus).await {
        Ok(true) => {
            tracing::info!("pairing_moved_listener: rapport {mr_id} rattaché à {to_round_id}")
        }
        Ok(false) => {}
        Err(e) => tracing::error!("pairing_moved_listener: rapport {mr_id}: {e:?}"),
    }
}
