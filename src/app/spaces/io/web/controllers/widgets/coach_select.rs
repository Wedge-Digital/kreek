use crate::app::shared_kernel::identity::coach_definition::CoachDefinition;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::spaces::context::SpacesContext;
use crate::common::initials::initials;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

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
    State(ctx): State<SpacesContext>,
) -> impl IntoResponse {
    let sid = match SpaceId::try_new(&query.space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let coaches = ctx
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

    CoachSelectorWidgetTemplate { coaches }.into_response()
}
