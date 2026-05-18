use askama::Template;
use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::web::app_layout::AppLayout;

#[derive(Template, Default)]
#[template(path = "new-competition-phase-1.html")]
pub struct NewCompetitionTemplate {
    pub space_id: String,
    pub logo_url_value:  String,
    pub logo_error:      Option<String>,
}

impl IntoResponse for NewCompetitionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_1(Path(space_id): Path<String>, headers: HeaderMap) -> impl IntoResponse {
    let tmpl = NewCompetitionTemplate { space_id, ..Default::default() };
    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}