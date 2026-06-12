use crate::app::shared_kernel::common_types::{SpaceId};
use crate::common::initials::initials;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::shared_kernel::coach_definition::CoachDefinition;

#[derive(Deserialize)]
pub struct CoachSelectorQueryParams {
    pub space_id: String,
}

#[derive(Template)]
#[template(path = "widgets/coach-select.html")]
pub struct CoachSelectorWidgetTemplate {
    pub coaches: Vec<CoachDefinition>,
}

impl IntoResponse for CoachSelectorWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}


pub async fn get_coach_selector_widget(
    Query(query): Query<CoachSelectorQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sid = match SpaceId::try_new(&query.space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let coaches = state
        .spaces
        .user_cache_repository
        .list_members_for_space(&sid)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|u| CoachDefinition {
            id: u.id,
            name: u.name.clone(),
            icon: u.icon,
            initials: initials(&u.name.to_string()),
        })
        .collect();

    CoachSelectorWidgetTemplate {
        coaches
    }
    .into_response()
}