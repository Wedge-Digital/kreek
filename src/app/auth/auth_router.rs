use crate::app::auth::io::web::get_auth_layout::auth_layout;
use crate::app::auth::io::web::get_login::login_form;
use crate::app::auth::io::web::get_login_success::login_success;
use crate::app::auth::io::web::get_register::get_register;
use crate::app::auth::io::web::get_register_success::register_success;
use crate::app::auth::io::web::post_login::login_submit;
use crate::app::auth::io::web::post_register::post_register;
use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth",                  get(auth_layout))
        .route("/auth/login",            get(login_form).post(login_submit))
        .route("/auth/login/success",    get(login_success))
        .route("/auth/register",         get(get_register).post(post_register))
        .route("/auth/register/success", get(register_success))
}
