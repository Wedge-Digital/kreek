use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::Entity;
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct DraftTeamCardVm {
    pub id: String,
    pub initials: String,
    pub name: String,
    pub logo: Option<String>,
    pub roster: Option<String>,
    pub link: String,
}

#[derive(Template)]
#[template(path = "my-teams.html")]
pub struct MyTeamsTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub drafts: Vec<DraftTeamCardVm>,
}

impl IntoResponse for MyTeamsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

async fn fetch_drafts(
    state: &AppState,
    coach_id: &str,
    space_id: &str,
) -> Result<Vec<DraftTeam>, StatusCode> {
    state
        .team_creation
        .team_repository
        .find_by_coach_and_space(coach_id, space_id)
        .await
        .map_err(|e| {
            tracing::error!("my_teams find_by_coach_and_space: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn fetch_submitted_ids(state: &AppState, space_id: &str) -> Vec<String> {
    state
        .team_creation
        .roster_repository
        .find_submitted_ids_for_space(space_id)
        .await
        .unwrap_or_default()
}

fn filter_unsubmitted(drafts: Vec<DraftTeam>, submitted_ids: &[String]) -> Vec<DraftTeam> {
    drafts
        .into_iter()
        .filter(|d| !submitted_ids.contains(&d.get_id().to_string()))
        .collect()
}

async fn fetch_roster_name(state: &AppState, team_id: &TeamId) -> Option<String> {
    state
        .team_creation
        .roster_repository
        .find_by_id(team_id)
        .await
        .ok()
        .flatten()
        .map(|t| t.roster.name.to_string())
}

async fn build_draft_vm(
    draft: DraftTeam,
    app_routes: &AppRoutes,
    space_id: &str,
    state: &AppState,
) -> DraftTeamCardVm {
    let team_id = draft.get_id();
    let id = team_id.to_string();
    let roster = fetch_roster_name(state, &team_id).await;
    let base = draft.base_infos();
    DraftTeamCardVm {
        initials: initials(base.name().as_ref()),
        name: base.name().clone().into_inner(),
        logo: base.logo_url().map(|u| u.as_ref().to_string()),
        roster,
        link: app_routes.team_creation.team_build(space_id, &id),
        id,
    }
}

async fn build_draft_vms(
    drafts: Vec<DraftTeam>,
    app_routes: &AppRoutes,
    space_id: &str,
    state: &AppState,
) -> Vec<DraftTeamCardVm> {
    let mut vms = Vec::new();
    for draft in drafts {
        vms.push(build_draft_vm(draft, app_routes, space_id, state).await);
    }
    vms
}

async fn load_my_teams_template(
    state: &AppState,
    coach_id: &str,
    space_id: String,
) -> Result<MyTeamsTemplate, StatusCode> {
    let drafts = fetch_drafts(state, coach_id, &space_id).await?;
    let submitted_ids = fetch_submitted_ids(state, &space_id).await;
    let drafts = filter_unsubmitted(drafts, &submitted_ids);
    let app_routes = AppRoutes::default();
    let drafts = build_draft_vms(drafts, &app_routes, &space_id, state).await;
    Ok(MyTeamsTemplate {
        app_routes,
        space_id,
        drafts,
    })
}

pub async fn my_teams(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match load_my_teams_template(&state, &user.id.to_string(), space_id).await {
        Ok(tpl) => tpl.into_response(),
        Err(status) => status.into_response(),
    }
}
