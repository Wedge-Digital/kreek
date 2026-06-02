use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn request_log(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let thread = std::thread::current().id();

    tracing::info!(?thread, %method, %path, "→ requête reçue");

    let response = next.run(request).await;

    tracing::info!(?thread, %method, %path, status = %response.status(), "← réponse envoyée");
    response
}
