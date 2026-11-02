//! Le widget « qui suit, qui saisit » d'un match —
//! `GET /app/{space_id}/realtime/matches/{match_report_id}/presence`
//! (carte 644).
//!
//! Gardé par `space_scope` (le match est de l'espace) puis par `match_role`
//! (ni admin ni coach du match : 403). Il **ne lit que le registre** : le rôle
//! affiché est celui que le serveur a inscrit à la déclaration, jamais une
//! valeur recalculée ici.
//!
//! Le demandeur n'y figure pas, sur aucun de ses onglets : l'URL est figée au
//! rendu de la page et ne connaît pas la connexion qui la charge, et l'on ne
//! s'annonce pas à soi-même.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::identity::ids::{EntityId, UserId};
use crate::state::AppState;
use crate::web::ports::MatchRole;
use crate::web::realtime::registry::PresentConnection;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::collections::BTreeMap;

/// Une personne présente, et ce qu'elle fait sur le match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceLineVm {
    pub coach_name: String,
    pub label: &'static str,
}

#[derive(Template)]
#[template(path = "realtime/presence-widget.html")]
pub struct PresenceWidgetTemplate {
    pub lines: Vec<PresenceLineVm>,
}

impl IntoResponse for PresenceWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("presence-widget.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

fn label(role: MatchRole) -> Option<&'static str> {
    match role {
        MatchRole::Write => Some("saisit"),
        MatchRole::Follow => Some("suit le match"),
        MatchRole::None => None,
    }
}

/// Les autres présents, une ligne par personne — son rôle le plus fort parmi
/// ses onglets —, triés par nom.
pub fn build_presence_lines(
    present: &[PresentConnection],
    requester: &UserId,
) -> Vec<PresenceLineVm> {
    let mut by_name: BTreeMap<String, MatchRole> = BTreeMap::new();
    for connection in present.iter().filter(|c| &c.user_id != requester) {
        let role = by_name
            .entry(connection.coach_name.to_string())
            .or_insert(MatchRole::None);
        *role = (*role).max(connection.role);
    }
    by_name
        .into_iter()
        .filter_map(|(coach_name, role)| {
            Some(PresenceLineVm {
                coach_name,
                label: label(role)?,
            })
        })
        .collect()
}

/// Les deux identifiants du chemin ; `space_scope` a déjà refusé un match
/// inconnu, un identifiant illisible n'arrive donc qu'ici.
fn parse_ids(space_id: &str, match_report_id: &str) -> Option<(EntityId, EntityId)> {
    Some((
        EntityId::try_new(space_id).ok()?,
        EntityId::try_new(match_report_id).ok()?,
    ))
}

pub async fn get_presence_widget(
    State(state): State<AppState>,
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some((space_id, match_report_id)) = parse_ids(&space_id, &match_report_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let right = state
        .match_role
        .match_role(&user, &space_id, &match_report_id)
        .await;
    if right == MatchRole::None {
        return StatusCode::FORBIDDEN.into_response();
    }
    let present = state.realtime.present_on(&match_report_id);
    PresenceWidgetTemplate {
        lines: build_presence_lines(&present, &user.id),
    }
    .into_response()
}
