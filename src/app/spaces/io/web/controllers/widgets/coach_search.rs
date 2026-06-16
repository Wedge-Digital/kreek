use crate::app::shared_kernel::common_types::{CoachId, SpaceId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::coach_definition::CoachDefinition;
use crate::app::spaces::io::web::controllers::widgets::coach_search_results::find_coaches;
use crate::common::initials::initials;

#[derive(Deserialize)]
pub struct CoachSearchWidgetParams {
    pub space_id: String,
    #[serde(default)]
    pub selected: String,
}

#[derive(Template)]
#[template(path = "widgets/coach-search.html")]
pub struct CoachSearchTemplate {
    pub routes: AppRoutes,
    pub space_id: String,
    pub selected_coaches: Vec<CoachDefinition>,
    pub coaches: Vec<CoachDefinition>,
    pub excluded: String,
}

impl IntoResponse for CoachSearchTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn search_coaches_controller(
    State(state): State<AppState>,
    Query(params): Query<CoachSearchWidgetParams>,
) -> impl IntoResponse {
    let space_id = match SpaceId::try_new(&params.space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let mut selected_coaches = Vec::new();
    for id_str in params.selected.split(',').filter(|s| !s.is_empty()) {
        if let Ok(coach_id) = CoachId::try_new(id_str) {
            if let Ok(user) = state.spaces.user_cache_repository.find_user_by_id(&coach_id).await {
                let name_str = user.name.to_string();
                let initials_str = initials(&name_str);
                selected_coaches.push(CoachDefinition {
                    id: user.id,
                    name: user.name,
                    icon: user.icon,
                    initials: initials_str,
                });
            }
        }
    }

    let excluded_set: std::collections::HashSet<String> = params
        .selected
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let coaches = find_coaches(&state, &space_id, "", &excluded_set).await;

    let excluded = params.selected.clone();

    CoachSearchTemplate {
        routes: AppRoutes::default(),
        space_id: params.space_id,
        selected_coaches,
        coaches,
        excluded,
    }.into_response()
}