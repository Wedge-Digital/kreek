//! L'appariement des présents (E18) — l'étape « Présents ».
//!
//! Sur une journée vide, l'organisateur décoche les équipes absentes ; rien
//! n'est écrit. Le tirage parmi les présentes est l'étape suivante (carte 617).

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::io::web::admin::admin_scope::journee_de_la_saison;
use crate::app::competitions::use_cases::admin::draw::draw_candidates_service::{
    self, CandidateGroup, DrawCandidates, DrawCandidatesError,
};
use crate::app::competitions::use_cases::admin::draw::draw_input::DrawDeps;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Deserialize)]
pub struct DrawQuery {
    pub round_id: String,
    /// Les absentes, en identifiants séparés par des virgules : le retour de
    /// l'aperçu (617) les restitue décochées.
    pub absent: Option<String>,
}

// ── View models ──────────────────────────────────────────────────────────────

pub struct DrawPresenceVm {
    pub round_id: String,
    pub groups: Vec<PresenceGroupVm>,
}

pub struct PresenceGroupVm {
    pub name: String,
    pub teams: Vec<PresenceTeamVm>,
    pub present_count: usize,
}

pub struct PresenceTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub matches_played: u32,
    pub present: bool,
}

impl DrawPresenceVm {
    /// Toutes les équipes sont présentes (R2), sauf celles que l'organisateur
    /// avait décochées.
    pub fn from_candidates(
        round_id: &str,
        candidates: &DrawCandidates,
        absent: &HashSet<String>,
    ) -> Self {
        Self {
            round_id: round_id.to_string(),
            groups: candidates
                .groups
                .iter()
                .map(|group| PresenceGroupVm::from_group(group, absent))
                .collect(),
        }
    }

    pub fn present_count(&self) -> usize {
        self.groups.iter().map(|g| g.present_count).sum()
    }
}

impl PresenceGroupVm {
    fn from_group(group: &CandidateGroup, absent: &HashSet<String>) -> Self {
        let teams: Vec<PresenceTeamVm> = group
            .teams
            .iter()
            .map(|c| PresenceTeamVm {
                team_id: c.team_id.clone(),
                team_name: c.team_name.clone(),
                coach_name: c.coach_name.clone(),
                roster_name: c.roster_name.clone(),
                matches_played: c.matches_played,
                present: !absent.contains(&c.team_id),
            })
            .collect();
        Self {
            name: group.group_name.clone(),
            present_count: teams.iter().filter(|t| t.present).count(),
            teams,
        }
    }
}

/// `absent=a,b` → `{a, b}` ; vide ou absent → aucun.
pub fn parse_absent(raw: Option<&str>) -> HashSet<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect()
}

// ── Gabarit ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "admin/widgets/schedule-draw-presence.html")]
pub struct DrawPresenceTemplate {
    pub draw: DrawPresenceVm,
}

impl IntoResponse for DrawPresenceTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("schedule draw presence render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ──────────────────────────────────────────────────────────────────

pub async fn get_draw_widget(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    auth_session: AuthSession,
    Query(query): Query<DrawQuery>,
) -> Response {
    let scope = AdminScope {
        space_id: &space_id,
        competition_id: &competition_id,
        season_id: &season_id,
    };
    if let Err(refus) = check_access(&auth_session, &scope, &query.round_id, &state).await {
        return refus;
    }
    match load_candidates(&query.round_id, &season_id, &state).await {
        Ok(candidates) => {
            let absent = parse_absent(query.absent.as_deref());
            let draw = DrawPresenceVm::from_candidates(&query.round_id, &candidates, &absent);
            DrawPresenceTemplate { draw }.into_response()
        }
        Err(e) => candidates_refused(e),
    }
}

struct AdminScope<'a> {
    space_id: &'a str,
    competition_id: &'a str,
    season_id: &'a str,
}

/// L'administration de la compétition, et une journée de cette saison.
async fn check_access(
    auth_session: &AuthSession,
    scope: &AdminScope<'_>,
    round_id: &str,
    state: &AppState,
) -> Result<(), Response> {
    require_admin_access(
        auth_session,
        scope.space_id,
        scope.competition_id,
        scope.season_id,
        state,
    )
    .await?;
    journee_de_la_saison(round_id, scope.season_id, state).await?;
    Ok(())
}

async fn load_candidates(
    round_id: &str,
    season_id: &str,
    state: &AppState,
) -> Result<DrawCandidates, DrawCandidatesError> {
    let deps = DrawDeps {
        match_day_repo: state.competitions.match_day_repository.as_ref(),
        group_repo: state.competitions.group_repository.as_ref(),
        team_port: state.competitions.team_info_port.as_ref(),
    };
    draw_candidates_service::load(round_id, season_id, &deps).await
}

#[derive(Serialize)]
struct ErrorResult {
    error: String,
}

/// Un refus métier se dit en 409 et `{ error }`, que
/// `handleScheduleActionResponse` rend en toast d'erreur.
fn candidates_refused(e: DrawCandidatesError) -> Response {
    match e {
        DrawCandidatesError::RoundNotFound => StatusCode::NOT_FOUND.into_response(),
        DrawCandidatesError::Refused(refus) => (
            StatusCode::CONFLICT,
            Json(ErrorResult {
                error: refus.to_string(),
            }),
        )
            .into_response(),
        DrawCandidatesError::Repository(msg) => {
            tracing::error!("get_draw_widget: {msg}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::use_cases::admin::draw::draw_candidates_service::Candidate;

    fn candidate(id: &str) -> Candidate {
        Candidate {
            team_id: id.to_string(),
            team_name: format!("Équipe {id}"),
            coach_name: "Coach".to_string(),
            roster_name: "Nains".to_string(),
            matches_played: 0,
        }
    }

    fn candidates() -> DrawCandidates {
        DrawCandidates {
            groups: vec![CandidateGroup {
                group_name: "A".to_string(),
                teams: vec![candidate("a"), candidate("b"), candidate("c")],
            }],
        }
    }

    #[test]
    fn every_team_is_present_by_default() {
        let vm = DrawPresenceVm::from_candidates("r", &candidates(), &HashSet::new());

        assert!(vm.groups[0].teams.iter().all(|t| t.present));
        assert_eq!(vm.present_count(), 3);
    }

    /// Le retour de l'aperçu restitue les décochages.
    #[test]
    fn absent_teams_come_back_unchecked() {
        let absent = parse_absent(Some("b, c"));

        let vm = DrawPresenceVm::from_candidates("r", &candidates(), &absent);

        let present: Vec<bool> = vm.groups[0].teams.iter().map(|t| t.present).collect();
        assert_eq!(present, vec![true, false, false]);
        assert_eq!(vm.groups[0].present_count, 1);
    }

    #[test]
    fn an_empty_absent_list_marks_no_one() {
        assert!(parse_absent(None).is_empty());
        assert!(parse_absent(Some(" , ")).is_empty());
    }
}
