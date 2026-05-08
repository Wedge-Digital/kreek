use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::spaces::routes::Routes;

#[derive(Template, Default)]
#[template(path = "app-spaces.html")]
pub struct AppSpaces {
    pub routes: Routes,
}

impl IntoResponse for AppSpaces {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn app_spaces() -> impl IntoResponse {
    AppSpaces::default()
}
