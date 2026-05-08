use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::web::routes::Routes;

#[derive(Template, Default)]
#[template(path = "app-menu.html")]
pub struct AppMenu {
    pub routes: Routes,
}

impl AppMenu {

}

impl IntoResponse for AppMenu {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn app_menu() -> impl IntoResponse {
    AppMenu::default()
}
