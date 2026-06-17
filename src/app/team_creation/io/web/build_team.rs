use crate::app::auth::auth_backend::AuthSession;
use crate::app::references::routes::Routes as RefRoutes;
use crate::app::shared_kernel::common_types::EntityId;
use crate::app::team_creation::io::web::view_models::{RulesPanelVm, RulesTierVm};
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::app::team_creation::use_cases::commands::SubmitTeamCommand;
use crate::app::team_creation::use_cases::submit_team as submit_uc;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── Page complète ─────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "build-team.html")]
pub struct BuildTeamTemplate {
    pub web_routes: WebRoutes,
    pub team_routes: TeamCreationRoutes,
    pub ref_routes: RefRoutes,
    pub space_id: String,
    pub team_id: String,
    pub rules_panel: RulesPanelVm,
}

impl IntoResponse for BuildTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn build_team(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let draft = match state
        .team_creation
        .team_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("build_team draft find {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let competition_name = if let Ok(id) = EntityId::try_new(draft.competition_id()) {
        state
            .competitions
            .competition_repository
            .find_base_info(&id)
            .await
            .ok()
            .flatten()
            .map(|i| i.name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let season_name = if let Ok(id) = EntityId::try_new(draft.season_id()) {
        state
            .competitions
            .season_repository
            .find_base_info(&id)
            .await
            .ok()
            .flatten()
            .map(|i| i.name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let rules_panel = RulesPanelVm {
        competition_name,
        season_name,
        tiers: draft
            .creation_rules()
            .tiers
            .iter()
            .map(|t| RulesTierVm {
                name: t.name.clone(),
                budget: t.budget,
                start_xp: t.start_xp,
            })
            .collect(),
    };

    BuildTeamTemplate {
        web_routes: Default::default(),
        team_routes: Default::default(),
        ref_routes: Default::default(),
        space_id,
        team_id,
        rules_panel,
    }
    .into_response()
}

// ── Handler submit_team ───────────────────────────────────────────────────────

pub async fn submit_team(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let cmd = SubmitTeamCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
        coach_name: user.coach_name.into_inner(),
    };

    match submit_uc::execute(
        cmd,
        state.team_creation.roster_repository.as_ref(),
        &state.team_creation.event_bus,
    )
    .await
    {
        Ok(()) => {}
        Err(submit_uc::SubmitTeamError::TeamNotFound) => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(submit_uc::SubmitTeamError::Domain(ref errors)) => {
            let msgs: String = errors
                .iter()
                .map(|e| {
                    format!(
                        r#"<p class="table-error">{}</p>"#,
                        submit_uc::domain_error_message(e)
                    )
                })
                .collect();
            return Response::builder()
                .header("HX-Retarget", "#submit-error")
                .header("HX-Reswap", "innerHTML")
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(msgs))
                .unwrap();
        }
        Err(submit_uc::SubmitTeamError::Repository(e)) => {
            tracing::error!("submit_team repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let team_routes: TeamCreationRoutes = Default::default();
    Response::builder()
        .header("HX-Redirect", team_routes.my_teams(&space_id))
        .header(
            "HX-Trigger",
            r#"{"showToast":"Équipe soumise avec succès !"}"#,
        )
        .body(Body::empty())
        .unwrap()
}
