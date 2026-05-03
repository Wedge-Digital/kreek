use axum::response::{ IntoResponse };
use crate::app::auth::io::web::auth_layout::AuthLayout;

pub async fn auth_layout() -> impl IntoResponse {
    AuthLayout {
        content: String::new()
    }
}
