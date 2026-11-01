//! L'appariement des présents (E18) — l'aperçu du tirage.
//!
//! « Tirer au sort » et « Retirer au sort » postent les présentes ; la réponse
//! est l'aperçu, swappé à la place de l'étape « Présents ». Rien n'est écrit.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::draw::History;
use crate::app::competitions::io::web::admin::schedule_draw_widget::{
    check_access, AdminScope, ErrorResult,
};
use crate::app::competitions::use_cases::admin::draw::draw_candidates_service::Candidate;
use crate::app::competitions::use_cases::admin::draw::draw_input::DrawDeps;
use crate::app::competitions::use_cases::admin::draw::preview_draw_use_case::{
    self, DrawPreview, GroupPreview, PreviewDrawCommand, PreviewDrawError, PreviewPairing,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use axum_extra::extract::Form;
use rand::SeedableRng;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PreviewDrawForm {
    pub round_id: String,
    #[serde(default)]
    pub team_ids: Vec<String>,
}

// ── View models ──────────────────────────────────────────────────────────────

pub struct DrawPreviewVm {
    pub round_id: String,
    pub groups: Vec<PreviewGroupVm>,
    /// Les présentes, rendues à « Retirer au sort ».
    pub present_ids: Vec<String>,
    /// Les absentes, rendues à « Modifier les présents ».
    pub absent_csv: String,
}

pub struct PreviewGroupVm {
    pub name: String,
    pub pairings: Vec<PreviewPairingVm>,
    pub exempted: Option<BenchTeamVm>,
    pub absent_names: Vec<String>,
    pub has_rematch: bool,
}

pub struct PreviewPairingVm {
    pub home_id: String,
    pub home_name: String,
    pub away_id: String,
    pub away_name: String,
    pub history_label: String,
    pub is_rematch: bool,
}

pub struct BenchTeamVm {
    pub team_id: String,
    pub name: String,
    pub matches_played: u32,
}

impl DrawPreviewVm {
    pub fn from_domain(round_id: &str, preview: &DrawPreview) -> Self {
        let absent: Vec<&Candidate> = preview.groups.iter().flat_map(|g| &g.absent).collect();
        Self {
            round_id: round_id.to_string(),
            groups: preview
                .groups
                .iter()
                .map(PreviewGroupVm::from_domain)
                .collect(),
            present_ids: preview.groups.iter().flat_map(present_ids).collect(),
            absent_csv: absent
                .iter()
                .map(|c| c.team_id.as_str())
                .collect::<Vec<_>>()
                .join(","),
        }
    }
}

/// Les présentes d'une poule : les tirées et l'exemptée.
fn present_ids(group: &GroupPreview) -> Vec<String> {
    group
        .pairings
        .iter()
        .flat_map(|p| [p.home.team_id.clone(), p.away.team_id.clone()])
        .chain(group.exempted.iter().map(|c| c.team_id.clone()))
        .collect()
}

impl PreviewGroupVm {
    fn from_domain(group: &GroupPreview) -> Self {
        let pairings: Vec<PreviewPairingVm> = group
            .pairings
            .iter()
            .map(PreviewPairingVm::from_domain)
            .collect();
        Self {
            name: group.group_name.clone(),
            has_rematch: pairings.iter().any(|p| p.is_rematch),
            pairings,
            exempted: group.exempted.as_ref().map(|c| BenchTeamVm {
                team_id: c.team_id.clone(),
                name: c.team_name.clone(),
                matches_played: c.matches_played,
            }),
            absent_names: group.absent.iter().map(|c| c.team_name.clone()).collect(),
        }
    }
}

impl PreviewPairingVm {
    fn from_domain(pairing: &PreviewPairing) -> Self {
        Self {
            home_id: pairing.home.team_id.clone(),
            home_name: pairing.home.team_name.clone(),
            away_id: pairing.away.team_id.clone(),
            away_name: pairing.away.team_name.clone(),
            history_label: history_label(&pairing.history),
            is_rematch: matches!(pairing.history, History::Rematch { .. }),
        }
    }
}

/// Ce que l'écran dit de l'historique que le domaine a retenu.
pub fn history_label(history: &History) -> String {
    match history {
        History::FirstMeeting => "1re rencontre".to_string(),
        History::Rematch { times, last_round } if times.0 <= 1 => {
            format!("Revanche · déjà jouée en {last_round}")
        }
        History::Rematch { times, last_round } => {
            format!(
                "Revanche · déjà jouée {} fois, dernière en {last_round}",
                times.0
            )
        }
    }
}

// ── Gabarit ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "admin/widgets/schedule-draw-preview.html")]
pub struct DrawPreviewTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub draw: DrawPreviewVm,
}

impl IntoResponse for DrawPreviewTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("schedule draw preview render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ──────────────────────────────────────────────────────────────────

pub async fn post_draw_preview(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(form): Form<PreviewDrawForm>,
) -> Response {
    let scope = AdminScope {
        space_id: &space_id,
        competition_id: &competition_id,
        season_id: &season_id,
    };
    if let Err(refus) = check_access(&auth_session, &scope, &form.round_id, &state).await {
        return refus;
    }
    let Some(cmd) = command_of(&form, &season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match preview(cmd, &state).await {
        Ok(preview) => preview_template(&scope, &form.round_id, &preview).into_response(),
        Err(e) => preview_refused(e),
    }
}

fn preview_template(
    scope: &AdminScope<'_>,
    round_id: &str,
    preview: &DrawPreview,
) -> DrawPreviewTemplate {
    DrawPreviewTemplate {
        app_routes: AppRoutes::default(),
        space_id: scope.space_id.to_string(),
        competition_id: scope.competition_id.to_string(),
        season_id: scope.season_id.to_string(),
        draw: DrawPreviewVm::from_domain(round_id, preview),
    }
}

/// Chaque identifiant passe par son type : un identifiant illisible est une
/// requête mal formée, pas une équipe à ignorer.
fn command_of(form: &PreviewDrawForm, season_id: &str) -> Option<PreviewDrawCommand> {
    Some(PreviewDrawCommand {
        round_id: MatchId::try_new(&form.round_id).ok()?,
        season_id: SeasonId::try_new(season_id).ok()?,
        present: form
            .team_ids
            .iter()
            .map(|id| TeamId::try_new(id).ok())
            .collect::<Option<Vec<_>>>()?,
    })
}

async fn preview(
    cmd: PreviewDrawCommand,
    state: &AppState,
) -> Result<DrawPreview, PreviewDrawError> {
    let deps = DrawDeps {
        match_day_repo: state.competitions.match_day_repository.as_ref(),
        group_repo: state.competitions.group_repository.as_ref(),
        team_port: state.competitions.team_info_port.as_ref(),
    };
    let mut rng = rand::rngs::StdRng::from_os_rng();
    preview_draw_use_case::execute(cmd, &deps, &mut rng).await
}

/// Un refus métier se dit en 409 et `{ error }`, que
/// `handleScheduleActionResponse` rend en toast d'erreur.
fn preview_refused(e: PreviewDrawError) -> Response {
    let message = match e {
        PreviewDrawError::RoundNotFound => return StatusCode::NOT_FOUND.into_response(),
        PreviewDrawError::IsRestDay => "Une journée de repos ne reçoit pas de rencontres.",
        PreviewDrawError::PairingsAlreadyExist => {
            "Cette journée a déjà des rencontres : rechargez-la pour voir son état."
        }
        PreviewDrawError::NotEnoughPresent => {
            "Il faut au moins deux équipes présentes dans une même poule pour tirer une rencontre."
        }
        // Le tirage respecte déjà R3 et R4 : un refus du domaine signale un
        // défaut, pas une saisie.
        PreviewDrawError::Refused(refus) => {
            tracing::error!("post_draw_preview : refus inattendu du domaine : {refus}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        PreviewDrawError::Repository(msg) => {
            tracing::error!("post_draw_preview : {msg}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    (
        StatusCode::CONFLICT,
        Json(ErrorResult {
            error: message.to_string(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::draw::MeetingCount;
    use crate::app::competitions::domain::match_day::MatchDayName;

    fn named(id: &str) -> Candidate {
        Candidate {
            team_id: id.to_string(),
            team_name: format!("Équipe {id}"),
            coach_name: String::new(),
            roster_name: String::new(),
            matches_played: 3,
        }
    }

    fn rematch(times: u32) -> History {
        History::Rematch {
            times: MeetingCount(times),
            last_round: MatchDayName::try_new("J1".to_string()).unwrap(),
        }
    }

    #[test]
    fn the_history_says_first_meeting_or_rematch() {
        assert_eq!(history_label(&History::FirstMeeting), "1re rencontre");
        assert_eq!(history_label(&rematch(1)), "Revanche · déjà jouée en J1");
        assert_eq!(
            history_label(&rematch(2)),
            "Revanche · déjà jouée 2 fois, dernière en J1"
        );
    }

    #[test]
    fn the_vm_keeps_present_and_absent_teams_for_the_next_steps() {
        let preview = DrawPreview {
            groups: vec![GroupPreview {
                group_name: "A".to_string(),
                pairings: vec![PreviewPairing {
                    home: named("a"),
                    away: named("b"),
                    history: rematch(1),
                }],
                exempted: Some(named("c")),
                absent: vec![named("d"), named("e")],
            }],
        };

        let vm = DrawPreviewVm::from_domain("r", &preview);

        assert_eq!(vm.present_ids, vec!["a", "b", "c"]);
        assert_eq!(vm.absent_csv, "d,e");
        assert!(vm.groups[0].has_rematch);
        assert_eq!(vm.groups[0].absent_names, vec!["Équipe d", "Équipe e"]);
    }
}
