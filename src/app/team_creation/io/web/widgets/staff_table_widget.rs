use crate::app::shared_kernel::common_types::EntityId;
use crate::app::shared_kernel::staff::StaffId;
use crate::app::team_creation::io::web::view_models::{RerollVm, StaffRowVm};
use crate::app::routes::AppRoutes;
use crate::app::team_creation::use_cases::build_team::buy_reroll as buy_reroll_uc;
use crate::app::team_creation::use_cases::build_team::buy_staff as buy_staff_uc;
use crate::app::team_creation::use_cases::commands::{
    BuyRerollCommand, BuyStaffCommand, RemoveRerollCommand, RemoveStaffCommand,
};
use crate::app::team_creation::use_cases::build_team::remove_reroll as remove_reroll_uc;
use crate::app::team_creation::use_cases::build_team::remove_staff as remove_staff_uc;
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn staff_error(msg: &str) -> Response {
    Response::builder()
        .header("HX-Retarget", "#staff-table-error")
        .header("HX-Reswap", "innerHTML")
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(format!(r#"<p class="table-error">{msg}</p>"#)))
        .unwrap()
}

// ── Widget staff table (GET) ─────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "widgets/staff-table-widget.html")]
pub struct StaffTableWidgetTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
    pub staff_rows: Vec<StaffRowVm>,
    pub reroll: Option<RerollVm>,
}

impl IntoResponse for StaffTableWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn staff_table_widget(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let (staff_rows, reroll, space_id) = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(team)) => (
            StaffRowVm::all_from_domain(&team),
            Some(RerollVm::from_domain(&team)),
            _space_id,
        ),
        _ => (vec![], None, _space_id),
    };

    StaffTableWidgetTemplate {
        app_routes: Default::default(),
        space_id,
        team_id,
        staff_rows,
        reroll,
    }
    .into_response()
}

// ── Fragment ligne staff ─────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "widgets/staff-row-fragment.html")]
pub struct StaffRowFragment {
    pub row: StaffRowVm,
    pub app_routes: AppRoutes,
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

// ── Fragment ligne relance ───────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "widgets/reroll-row-fragment.html")]
pub struct RerollRowFragment {
    pub reroll: RerollVm,
    pub app_routes: AppRoutes,
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

// ── Handler buy_staff ────────────────────────────────────────────────────────

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
        app_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}

// ── Handler remove_staff ─────────────────────────────────────────────────────

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
        app_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}

// ── Handler buy_reroll ───────────────────────────────────────────────────────

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
        app_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}

// ── Handler remove_reroll ────────────────────────────────────────────────────

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
        app_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}
