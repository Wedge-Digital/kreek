use crate::app::routes::AppRoutes;
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Sélecteur de la zone de contenu de `app-layout.html`. Les BCs extractibles
/// le reçoivent par injection plutôt que de le coder en dur dans leurs liens.
pub const CONTENT_TARGET: &str = "#app-content";

/// Enveloppe un fragment déjà rendu dans le layout de l'application.
///
/// Les pages d'un BC extractible ne peuvent pas faire `{% extends %}` sur le
/// layout du host — Askama résout l'héritage statiquement. Elles rendent un
/// fragment, que cette coquille place dans le bloc de contenu du layout.
#[derive(Template)]
#[template(path = "app-shell.html")]
pub struct AppShell {
    pub app_routes: AppRoutes,
    pub content: String,
}

impl IntoResponse for AppShell {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
