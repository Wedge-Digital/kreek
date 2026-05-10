use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::routes::path;

pub async fn require_auth(
    auth_session: AuthSession,
    request: Request<Body>,
    next: Next,
) -> Response {
    if auth_session.user.is_some() {
        return next.run(request).await;
    }

    // Requête HTMX — le navigateur ne suit pas les redirections HTTP,
    // il faut passer par l'en-tête HX-Redirect.
    if request.headers().contains_key("hx-request") {
        return Response::builder()
            .header("HX-Redirect", path::LOGIN)
            .body(Body::empty())
            .unwrap();
    }

    Redirect::to(path::LOGIN).into_response()
}