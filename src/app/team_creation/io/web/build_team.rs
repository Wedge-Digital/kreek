use crate::app::auth::auth_backend::AuthSession;
use crate::app::references::routes::Routes as RefRoutes;
use crate::app::shared_kernel::common_types::EntityId;
use crate::app::shared_kernel::staff::StaffId;
use crate::app::team_creation::io::web::view_models::{
    RerollVm, RulesPanelVm, RulesTierVm, StaffRowVm,
};
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::app::team_creation::use_cases::buy_reroll as buy_reroll_uc;
use crate::app::team_creation::use_cases::buy_staff as buy_staff_uc;
use crate::app::team_creation::use_cases::commands::{
    BuyRerollCommand, BuyStaffCommand, RemoveRerollCommand, RemoveStaffCommand, SubmitTeamCommand,
};
use crate::app::team_creation::use_cases::remove_reroll as remove_reroll_uc;
use crate::app::team_creation::use_cases::remove_staff as remove_staff_uc;
use crate::app::team_creation::use_cases::submit_team as submit_uc;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// ── Helpers de réponse d'erreur HTMX ─────────────────────────────────────────

fn staff_error(msg: &str) -> Response {
    Response::builder()
        .header("HX-Retarget", "#staff-table-error")
        .header("HX-Reswap", "innerHTML")
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(format!(r#"<p class="table-error">{msg}</p>"#)))
        .unwrap()
}

// ── Page complète ─────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "build-team.html")]
pub struct BuildTeamTemplate {
    pub web_routes: WebRoutes,
    pub team_routes: TeamCreationRoutes,
    pub ref_routes: RefRoutes,
    pub space_id: String,
    pub team_id: String,
    pub staff_rows: Vec<StaffRowVm>,
    pub reroll: Option<RerollVm>,
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

    let roster_team = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(opt) => opt,
        Err(e) => {
            tracing::error!("build_team roster find {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let (staff_rows, reroll) = match &roster_team {
        None => (vec![], None),
        Some(team) => {
            let staff = StaffRowVm::all_from_domain(team);
            let rv = RerollVm::from_domain(team);
            (staff, Some(rv))
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
        staff_rows,
        reroll,
        rules_panel,
    }
    .into_response()
}

// ── Fragment ligne staff ──────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "staff-row-fragment.html")]
pub struct StaffRowFragment {
    pub row: StaffRowVm,
    pub team_routes: TeamCreationRoutes,
    pub space_id: String,
    pub team_id: String,
}

impl IntoResponse for StaffRowFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .header("HX-Trigger", "teamMutated")
                .body(Body::from(html))
                .unwrap(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Fragment ligne relance ────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "reroll-row-fragment.html")]
pub struct RerollRowFragment {
    pub reroll: RerollVm,
    pub team_routes: TeamCreationRoutes,
    pub space_id: String,
    pub team_id: String,
}

impl IntoResponse for RerollRowFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .header("HX-Trigger", "teamMutated")
                .body(Body::from(html))
                .unwrap(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Handler buy_staff ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StaffBody {
    pub staff_id: String,
}

pub async fn buy_staff(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<StaffBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = BuyStaffCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
        staff_id: StaffId(body.staff_id.clone()),
    };

    let updated_team =
        match buy_staff_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
            Ok(t) => t,
            Err(buy_staff_uc::BuyStaffError::TeamNotFound) => {
                return StatusCode::NOT_FOUND.into_response()
            }
            Err(buy_staff_uc::BuyStaffError::StaffNotFoundInRoster) => {
                return StatusCode::UNPROCESSABLE_ENTITY.into_response()
            }
            Err(buy_staff_uc::BuyStaffError::Domain(ref errors)) => {
                return staff_error(buy_staff_uc::domain_error_message(errors))
            }
            Err(buy_staff_uc::BuyStaffError::Repository(e)) => {
                tracing::error!("buy_staff repo error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let staff_rows = StaffRowVm::all_from_domain(&updated_team);
    let row = match staff_rows.into_iter().find(|r| r.id == body.staff_id) {
        Some(r) => r,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    StaffRowFragment {
        row,
        team_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}

// ── Handler remove_staff ──────────────────────────────────────────────────────

pub async fn remove_staff(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<StaffBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = RemoveStaffCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
        staff_id: StaffId(body.staff_id.clone()),
    };

    let updated_team =
        match remove_staff_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
            Ok(t) => t,
            Err(remove_staff_uc::RemoveStaffError::TeamNotFound) => {
                return StatusCode::NOT_FOUND.into_response()
            }
            Err(remove_staff_uc::RemoveStaffError::StaffNotFoundInRoster) => {
                return StatusCode::UNPROCESSABLE_ENTITY.into_response()
            }
            Err(remove_staff_uc::RemoveStaffError::Domain(ref e)) => {
                return staff_error(remove_staff_uc::domain_error_message(e))
            }
            Err(remove_staff_uc::RemoveStaffError::Repository(e)) => {
                tracing::error!("remove_staff repo error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let staff_rows = StaffRowVm::all_from_domain(&updated_team);
    let row = match staff_rows.into_iter().find(|r| r.id == body.staff_id) {
        Some(r) => r,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    StaffRowFragment {
        row,
        team_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}

// ── Handler buy_reroll ────────────────────────────────────────────────────────

pub async fn buy_reroll(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = BuyRerollCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
    };

    let updated_team =
        match buy_reroll_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
            Ok(t) => t,
            Err(buy_reroll_uc::BuyRerollError::TeamNotFound) => {
                return StatusCode::NOT_FOUND.into_response()
            }
            Err(buy_reroll_uc::BuyRerollError::Domain(ref errors)) => {
                return staff_error(buy_reroll_uc::domain_error_message(errors))
            }
            Err(buy_reroll_uc::BuyRerollError::Repository(e)) => {
                tracing::error!("buy_reroll repo error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let reroll = RerollVm::from_domain(&updated_team);
    RerollRowFragment {
        reroll,
        team_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}

// ── Handler remove_reroll ─────────────────────────────────────────────────────

pub async fn remove_reroll(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = RemoveRerollCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
    };

    let updated_team = match remove_reroll_uc::execute(
        cmd,
        state.team_creation.roster_repository.as_ref(),
    )
    .await
    {
        Ok(t) => t,
        Err(remove_reroll_uc::RemoveRerollError::TeamNotFound) => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(remove_reroll_uc::RemoveRerollError::Repository(e)) => {
            tracing::error!("remove_reroll repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let reroll = RerollVm::from_domain(&updated_team);
    RerollRowFragment {
        reroll,
        team_routes: Default::default(),
        space_id,
        team_id,
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
