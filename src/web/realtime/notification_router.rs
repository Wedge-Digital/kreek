//! Le routage des notifications de match (carte 645, épic E23).
//!
//! Un listener **cross-BC** de l'`app_event_bus` — `init(app_event_bus: …)`,
//! la convention de l'axe 5 : il reçoit des faits déjà commités par
//! `match_report`, et n'écrit rien.
//!
//! Pour **tout** fait de match, un signal « le match X a changé » part
//! d'abord vers les connexions présentes sur ce match, auteur compris (carte
//! 646, R4) : la page recharge ses widgets par leurs routes, qui gardent leurs
//! droits. Aucun port n'est appelé pour cela — la présence (644) n'est
//! inscrite qu'avec le droit de suivre.
//!
//! Pour un fait qui fait un toast (décision 7) : destinataires = les coachs des
//! deux équipes, moins l'auteur (R2), croisés avec le registre, livrés à
//! **toutes** leurs connexions — y compris l'onglet qui regarde ce match
//! (décision 13). Un destinataire sans connexion ne le recevra jamais (R3).
//!
//! # Ce qui ne doit pas faire décrocher le bus
//!
//! Un seul appel de port par fait, aucun par connexion, et des livraisons par
//! `try_send` qui n'attendent jamais (R8). Si le routeur décroche malgré tout
//! (`Lagged`), les toasts perdus le restent ; les connexions reçoivent
//! `resync`, qui fait recharger les widgets abonnés.

use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportAppEvent;
use crate::app::shared_kernel::identity::ids::{EntityId, UserId};
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use crate::web::ports::IMatchAudiencePort;
use crate::web::realtime::message::RealtimeMessage;
use crate::web::realtime::registry::ConnectionRegistry;
use crate::web::realtime::toast::{notice_of, MatchNotice, TOAST_EVENT_TYPES};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;

/// Ce qu'a donné le routage d'un fait, tel que le journal le dit.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RoutingOutcome {
    /// Coachs du match, auteur exclu.
    pub recipients: usize,
    /// Parmi eux, ceux qui avaient au moins un onglet ouvert.
    pub connected: usize,
    /// Onglets dont la file a accepté le toast.
    pub delivered: usize,
}

pub fn init(
    app_event_bus: &EventBus,
    registry: Arc<ConnectionRegistry>,
    audience: Arc<dyn IMatchAudiencePort>,
) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => handle(&envelope, &registry, audience.as_ref()).await,
                Err(RecvError::Lagged(n)) => {
                    on_lagged(n, &registry);
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

/// Un app event reçu. Un fait de match signale d'abord ses présents (646),
/// puis fait son toast s'il en porte un (645) ; les faits des autres BCs sont
/// ignorés.
pub async fn handle(
    envelope: &EventEnvelope,
    registry: &ConnectionRegistry,
    audience: &dyn IMatchAudiencePort,
) {
    let Some(event) = decode(envelope) else {
        return;
    };
    // Le signal d'abord, et sans `await` : il n'attend pas le port du toast.
    signal(&event, envelope, registry);
    if let Some(notice) = notice_of(&event) {
        toast(envelope, &notice, registry, audience).await;
    }
}

/// Le fait de match d'une enveloppe, ou `None` pour un fait d'un autre BC.
///
/// Un échec de lecture est silencieux, comme dans tout listener de
/// `MatchReportAppEvent` — sauf pour un `event_type` qui fait un toast : ce
/// fait-là devait être lu, son échec se journalise.
fn decode(envelope: &EventEnvelope) -> Option<MatchReportAppEvent> {
    match serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone()) {
        Ok(event) => Some(event),
        Err(e) => {
            if TOAST_EVENT_TYPES.contains(&envelope.event_type.as_str()) {
                tracing::warn!(event_id = %envelope.event_id, error = %e, "fait de match illisible");
            }
            None
        }
    }
}

/// « Le match X a changé », à toutes les connexions présentes sur ce match,
/// auteur compris (carte 646). Pas de ligne propre au journal : chaque
/// livraison est comptée dans les `delivered` de sa connexion. Rend le nombre
/// de files qui ont accepté le signal.
pub fn signal(
    event: &MatchReportAppEvent,
    envelope: &EventEnvelope,
    registry: &ConnectionRegistry,
) -> usize {
    match EntityId::try_new(event.match_report_id()) {
        Ok(match_report_id) => registry.notify_match_changed(&match_report_id),
        Err(_) => {
            tracing::warn!(event_id = %envelope.event_id, "match illisible, signal abandonné");
            0
        }
    }
}

/// Le toast d'un fait, routé puis journalisé.
async fn toast(
    envelope: &EventEnvelope,
    notice: &MatchNotice,
    registry: &ConnectionRegistry,
    audience: &dyn IMatchAudiencePort,
) {
    let Ok(author) = UserId::try_new(&notice.author) else {
        // Abandonné plutôt que routé sans exclusion : l'auteur recevrait son
        // propre toast.
        tracing::warn!(event_id = %envelope.event_id, "auteur illisible, notification abandonnée");
        return;
    };
    let outcome = route(notice, &author, registry, audience).await;
    tracing::info!(
        event_id = %envelope.event_id,
        match_report_id = %notice.match_report_id,
        recipients = outcome.recipients,
        connected = outcome.connected,
        delivered = outcome.delivered,
        "notification routée"
    );
}

/// Livre le toast aux coachs du match, auteur exclu, sur tous leurs onglets.
pub async fn route(
    notice: &MatchNotice,
    author: &UserId,
    registry: &ConnectionRegistry,
    audience: &dyn IMatchAudiencePort,
) -> RoutingOutcome {
    let coaches = audience
        .coaches_of_match(&notice.space_id, &notice.home_team_id, &notice.away_team_id)
        .await;
    let message = RealtimeMessage::Toast(notice.toast.clone());
    let mut outcome = RoutingOutcome::default();
    for coach in coaches.iter().filter(|coach| *coach != author) {
        let delivery = registry.deliver_to_user(coach, &message);
        outcome.recipients += 1;
        outcome.connected += usize::from(delivery.connections > 0);
        outcome.delivered += delivery.queued;
    }
    outcome
}

/// Le routeur a décroché de `n` messages : toutes les connexions reçoivent
/// `resync`. Rend le nombre de files qui l'ont accepté.
///
/// Toutes, et non les seules présentes sur un match : un `realtimeResync` ne
/// recharge que les widgets qui s'y abonnent, et la présence (644) n'est pas
/// nécessaire pour le savoir.
pub fn on_lagged(n: u64, registry: &ConnectionRegistry) -> usize {
    let resynced = registry.deliver_to_all(&RealtimeMessage::Resync);
    tracing::warn!(
        n,
        resynced,
        "notifications perdues, resynchronisation demandée"
    );
    resynced
}
