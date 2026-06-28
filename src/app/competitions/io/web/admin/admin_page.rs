use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::dashboard::build_dashboard_fragment;
use crate::app::competitions::io::web::admin::summary_tab::build_summary_fragment;
use crate::app::competitions::use_cases::admin::dashboard_query;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId, SpaceId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin-page.html")]
pub struct AdminPageTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub competition_name: String,
    pub season_name: String,
    pub admin_count: usize,
    pub has_groups: bool,
    pub active_tab: String,
    pub content: String,
}

impl IntoResponse for AdminPageTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("admin page template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn admin_page(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    render_admin_page(auth_session, &space_id, &competition_id, &season_id, "dashboard", &state).await
}

pub async fn render_admin_page(
    auth_session: AuthSession,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    active_tab: &str,
    state: &AppState,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let comp_id = match CompetitionId::try_new(competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let space_entity_id = match SpaceId::try_new(space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let is_space_admin = matches!(
        state.spaces.space_repository.find_member_profile(&user.id, &space_entity_id).await,
        Ok(Some(SpaceProfile::SpaceAdmin))
    );

    let comp_info = match state
        .competitions
        .competition_repository
        .find_base_info(&comp_id)
        .await
    {
        Ok(Some(info)) => info,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("admin_page competition find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let user_id_str = user.id.to_string();
    let coach_name_str = user.coach_name.clone().into_inner();
    let is_comp_admin = comp_info.admin_ids.contains(&user_id_str)
        || comp_info.admin_names.contains(&coach_name_str);

    if !is_space_admin && !is_comp_admin {
        return StatusCode::FORBIDDEN.into_response();
    }

    let season_entity_id = match SeasonId::try_new(season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let season_name = state
        .competitions
        .season_repository
        .find_base_info(&season_entity_id)
        .await
        .ok()
        .flatten()
        .map(|i| i.name)
        .unwrap_or_default();

    let has_groups = state
        .competitions
        .season_repository
        .find_structure(&season_entity_id)
        .await
        .ok()
        .flatten()
        .map(|s| s.ranking_group.use_ranking_groups.0 && s.ranking_group.ranking_groups.len() > 1)
        .unwrap_or(false);

    let app_routes = AppRoutes::default();

    let content = match active_tab {
        "enrollments" => {
            let requires_validation = state
                .competitions
                .season_repository
                .find_invitations(&season_entity_id)
                .await
                .ok()
                .flatten()
                .map(|inv| inv.requires_validation.0)
                .unwrap_or(true);

            let tpl = super::enrollments_tab::EnrollmentsTabTemplate {
                app_routes,
                space_id: space_id.to_string(),
                competition_id: competition_id.to_string(),
                season_id: season_id.to_string(),
                requires_validation,
            };
            tpl.render().unwrap_or_default()
        }
        "summary" => {
            match build_summary_fragment(
                &comp_id,
                &season_entity_id,
                state.competitions.competition_repository.as_ref(),
                state.competitions.season_repository.as_ref(),
                state.references.repository.as_ref(),
                app_routes,
                space_id,
                competition_id,
                season_id,
            )
            .await
            {
                Some(tpl) => tpl.render().unwrap_or_default(),
                None => {
                    tracing::error!("admin_page summary build failed for {competition_id}/{season_id}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
        "groups" => {
            let tpl = super::groups_tab::GroupsTabTemplate {
                app_routes,
                space_id: space_id.to_string(),
                competition_id: competition_id.to_string(),
                season_id: season_id.to_string(),
            };
            tpl.render().unwrap_or_default()
        }
        "results" => {
            let tpl = super::results_tab::ResultsTabTemplate {
                app_routes,
                space_id: space_id.to_string(),
                competition_id: competition_id.to_string(),
                season_id: season_id.to_string(),
            };
            tpl.render().unwrap_or_default()
        }
        "schedule" => {
            let tpl = super::schedule_tab::ScheduleTabTemplate {
                app_routes,
                space_id: space_id.to_string(),
                competition_id: competition_id.to_string(),
                season_id: season_id.to_string(),
            };
            tpl.render().unwrap_or_default()
        }
        _ => {
            let summary = match dashboard_query::execute(
                &comp_id,
                &season_entity_id,
                state.competitions.competition_repository.as_ref(),
                state.competitions.season_repository.as_ref(),
            )
            .await
            {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("admin_page dashboard_query error: {e:?}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            let dashboard = build_dashboard_fragment(
                &summary, &app_routes, space_id, competition_id, season_id,
            );
            dashboard.render().unwrap_or_default()
        }
    };

    AdminPageTemplate {
        app_routes,
        space_id: space_id.to_string(),
        competition_id: competition_id.to_string(),
        season_id: season_id.to_string(),
        competition_name: comp_info.name,
        season_name,
        admin_count: comp_info.admin_ids.len(),
        has_groups,
        active_tab: active_tab.to_string(),
        content,
    }
    .into_response()
}
