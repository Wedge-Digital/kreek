use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::routes::path;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};

pub async fn require_auth(
    auth_session: AuthSession,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();

    if auth_session.user.is_some() {
        tracing::debug!(%path, "require_auth: OK — utilisateur authentifié");
        return next.run(request).await;
    }

    tracing::debug!(%path, "require_auth: non authentifié — redirection");

    if request.headers().contains_key("hx-request") {
        return Response::builder()
            .header("HX-Redirect", path::LOGIN)
            .body(Body::empty())
            .unwrap();
    }

    Redirect::to(path::LOGIN).into_response()
}
