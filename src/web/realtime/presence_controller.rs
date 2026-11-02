//! La déclaration de présence d'un onglet — `POST /app/realtime/presence`
//! (carte 644).
//!
//! Le script du layout l'envoie au `hello` et après chaque navigation htmx,
//! d'après le marqueur `.realtime-presence` de la page : le match regardé et
//! l'activité de la page (`follow` sur `/live`, `write` sur une page de
//! saisie), ou rien quand la page n'est plus une page de match.
//!
//! # Le rôle inscrit n'est jamais celui que la page prétend
//!
//! Le serveur calcule le **droit** par `match_role`, et inscrit le minimum du
//! droit et de l'activité déclarée. La page peut donc se déclarer en dessous
//! de son droit — un admin qui lit `/live` « suit le match » —, jamais
//! au-dessus. Un droit `None` est un 403, quelle que soit l'activité.
//!
//! # L'onglet doit être celui du demandeur
//!
//! L'identifiant de connexion vient du navigateur. Une connexion inconnue ou
//! appartenant à un autre utilisateur reçoit le même 404 : rien n'est révélé
//! des onglets d'autrui, et personne ne pose de présence à la place d'un autre.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::state::AppState;
use crate::web::ports::{IMatchRolePort, MatchRole};
use crate::web::realtime::registry::{
    ConnectionId, ConnectionRegistry, Presence, PresenceChange, PresenceError,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

/// Le formulaire tel que le layout l'envoie ; les trois derniers champs sont
/// vides pour un retrait.
#[derive(Debug, Deserialize)]
pub struct PresenceForm {
    pub connection_id: String,
    #[serde(default)]
    pub space_id: String,
    #[serde(default)]
    pub match_report_id: String,
    #[serde(default)]
    pub activity: String,
}

/// Le match visé et l'activité que la page déclare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresenceTarget {
    pub space_id: EntityId,
    pub match_report_id: EntityId,
    pub activity: MatchRole,
}

/// La déclaration validée : un onglet, et un match ou aucun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresenceRequest {
    pub connection_id: ConnectionId,
    pub target: Option<PresenceTarget>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PresenceRefusal {
    /// Ni admin ni coach du match, ou match hors de l'espace : 403.
    Forbidden,
    /// Onglet inconnu ou d'un autre utilisateur : 404.
    UnknownConnection,
}

/// L'activité déclarée par la page ; une valeur inconnue est illisible.
pub fn parse_activity(s: &str) -> Option<MatchRole> {
    match s {
        "follow" => Some(MatchRole::Follow),
        "write" => Some(MatchRole::Write),
        _ => None,
    }
}

impl PresenceForm {
    /// `None` quand un champ est illisible, ou qu'un match est donné à moitié.
    pub fn parse(&self) -> Option<PresenceRequest> {
        let connection_id = ConnectionId::parse(&self.connection_id)?;
        let empty =
            self.space_id.is_empty() && self.match_report_id.is_empty() && self.activity.is_empty();
        let target = if empty {
            None
        } else {
            Some(self.parse_target()?)
        };
        Some(PresenceRequest {
            connection_id,
            target,
        })
    }

    fn parse_target(&self) -> Option<PresenceTarget> {
        Some(PresenceTarget {
            space_id: EntityId::try_new(&self.space_id).ok()?,
            match_report_id: EntityId::try_new(&self.match_report_id).ok()?,
            activity: parse_activity(&self.activity)?,
        })
    }
}

/// Le rôle à inscrire : le minimum du droit et de l'activité, ou un refus
/// quand le droit est nul.
pub fn role_to_record(right: MatchRole, activity: MatchRole) -> Option<MatchRole> {
    if right == MatchRole::None {
        return None;
    }
    Some(right.min(activity))
}

/// Calcule la présence et la range dans le registre — sans HTTP, pour se
/// tester avec une doublure du port.
pub async fn declare_presence(
    registry: &ConnectionRegistry,
    port: &dyn IMatchRolePort,
    user: &User,
    request: &PresenceRequest,
) -> Result<(PresenceChange, Option<Presence>), PresenceRefusal> {
    let presence = match request.target {
        Some(target) => Some(resolve_presence(port, user, &target).await?),
        None => None,
    };
    let change = registry
        .set_presence(&request.connection_id, &user.id, presence)
        .map_err(|PresenceError::UnknownConnection| PresenceRefusal::UnknownConnection)?;
    Ok((change, presence))
}

async fn resolve_presence(
    port: &dyn IMatchRolePort,
    user: &User,
    target: &PresenceTarget,
) -> Result<Presence, PresenceRefusal> {
    let right = port
        .match_role(user, &target.space_id, &target.match_report_id)
        .await;
    let role = role_to_record(right, target.activity).ok_or(PresenceRefusal::Forbidden)?;
    Ok(Presence {
        match_report_id: target.match_report_id,
        role,
    })
}

pub async fn post_presence(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(form): Form<PresenceForm>,
) -> Response {
    // `require_auth` passe avant : ce cas ne se présente que si la route
    // sortait un jour de sa garde.
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(request) = form.parse() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let registry = state.realtime.as_ref();
    match declare_presence(registry, state.match_role.as_ref(), &user, &request).await {
        Ok((change, presence)) => announce(registry, &request, &change, presence),
        Err(PresenceRefusal::Forbidden) => StatusCode::FORBIDDEN.into_response(),
        Err(PresenceRefusal::UnknownConnection) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Journalise la présence et prévient les présents des matchs concernés.
fn announce(
    registry: &ConnectionRegistry,
    request: &PresenceRequest,
    change: &PresenceChange,
    presence: Option<Presence>,
) -> Response {
    let conn = request.connection_id;
    match presence {
        Some(p) => {
            tracing::debug!(%conn, match_report_id = %p.match_report_id, role = ?p.role, "présence")
        }
        None => tracing::debug!(%conn, role = "none", "présence"),
    }
    for match_report_id in &change.matches_to_notify {
        registry.notify_presence_changed(match_report_id, Some(conn));
    }
    StatusCode::NO_CONTENT.into_response()
}
