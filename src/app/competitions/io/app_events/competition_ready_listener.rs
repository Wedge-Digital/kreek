//! Souscrit à `CompetitionReady` pour annoncer l'ouverture des inscriptions.
//!
//! # Un listener, pas un `tokio::spawn` dans le handler
//!
//! `execute_finalize` émet déjà cet évènement sur le bus **interne** du BC. Le
//! handler HTTP reste ainsi un traducteur de protocole pur, et le détachement —
//! ne pas faire attendre la réponse pendant trente envois d'e-mail — est acquis
//! par le bus plutôt que bricolé sur place.
//!
//! # `init(event_bus: …)`, sans préfixe `app_`
//!
//! C'est le bus **interne**, et l'axe 5 de `check-arch` lit exactement cette
//! signature pour distinguer un listener intra-BC — qui pourrait partager une
//! transaction — d'un listener cross-BC, qui ne le peut pas.

use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::io::repository::notification_delivery_repository::NotificationDeliveryRepository;
use crate::app::competitions::io::repository::season_repository::SeasonRepository;
use crate::app::competitions::ports::{ICompetitionSpaceMemberPort, ITeamInfoPort};
use crate::app::competitions::use_cases::notification_dispatch::DispatchDeps;
use crate::app::competitions::use_cases::send_registration_open_use_case::{
    self, SendRegistrationOpenCommand,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::common::services::email::IEmailService;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use std::sync::Arc;
use time::macros::format_description;
use time::OffsetDateTime;
use tracing::Instrument;

pub struct RegistrationOpenDeps {
    pub pool: PgPool,
    pub teams: Arc<dyn ITeamInfoPort>,
    pub members: Arc<dyn ICompetitionSpaceMemberPort>,
    pub email: Arc<dyn IEmailService>,
    pub app_url: String,
}

pub fn init(event_bus: &EventBus, deps: Arc<RegistrationOpenDeps>) {
    let mut rx = event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<CompetitionsDomainEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "domain_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    handle_event(event, &deps).instrument(span).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::competition_ready_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(event: CompetitionsDomainEvent, deps: &RegistrationOpenDeps) {
    let CompetitionsDomainEvent::CompetitionReady {
        competition_id,
        space_id,
        ..
    } = event
    else {
        return;
    };

    let seasons = SeasonRepository::new(deps.pool.clone());
    let Ok(Some(season_id)) = seasons.find_latest_season_id(&competition_id).await else {
        tracing::warn!(competition = %competition_id, "aucune saison à annoncer");
        return;
    };
    annoncer(&seasons, &season_id, &space_id.to_string(), deps).await;
}

async fn annoncer(
    seasons: &SeasonRepository,
    season_id: &SeasonId,
    space_id: &str,
    deps: &RegistrationOpenDeps,
) {
    let invitations = seasons.find_invitations(season_id).await.ok().flatten();
    let base = seasons.find_base_info(season_id).await.ok().flatten();
    let journal = NotificationDeliveryRepository::new(deps.pool.clone());

    let reglages = seasons
        .find_notifications(season_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    // Le réglage commande ici comme ailleurs : décoché, rien ne part.
    if !reglages.registration_open.0 {
        return;
    }

    let dispatch_deps = DispatchDeps {
        teams: deps.teams.as_ref(),
        members: deps.members.as_ref(),
        journal: &journal,
        email: deps.email.as_ref(),
        app_url: &deps.app_url,
    };
    let cmd = SendRegistrationOpenCommand {
        season_id: season_id.to_string(),
        space_id: space_id.to_string(),
        competition_name: base.as_ref().map(|b| b.name.clone()).unwrap_or_default(),
        season_name: base.map(|b| b.name).unwrap_or_default(),
        opened_on: DateString::try_new(&aujourdhui()).unwrap_or_default(),
    };
    send_registration_open_use_case::execute(cmd, invitations.as_ref(), &dispatch_deps).await;
}

fn aujourdhui() -> String {
    OffsetDateTime::now_utc()
        .date()
        .format(format_description!("[year]-[month]-[day]"))
        .unwrap_or_default()
}
