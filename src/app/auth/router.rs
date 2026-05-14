use crate::app::auth::io::web::get_login::login_form;
use crate::app::auth::io::web::get_login_success::login_success;
use crate::app::auth::io::web::get_register::get_register;
use crate::app::auth::io::web::get_register_success::register_success;
use crate::app::auth::io::web::post_login::login_submit;
use crate::app::auth::io::web::post_logout::logout;
use crate::app::auth::io::web::post_register::post_register;
use crate::app::auth::routes::path;
use crate::state::AppState;
use axum::{routing::get, Router};
use crate::app::auth::io::web::auth_layout::auth_layout;
use crate::app::auth::io::web::forgot_password::{
    display_forgot_password, display_forgot_password_sent, post_forgot_password,
};
use crate::app::auth::io::web::reset_password::{display_reset_password, post_reset_password};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::AUTH_LAYOUT,              get(auth_layout))
        .route(path::LOGIN,                    get(login_form).post(login_submit))
        .route(path::LOGIN_SUCCESS,            get(login_success))
        .route(path::LOGOUT,                   get(logout))
        .route(path::REGISTER,                 get(get_register).post(post_register))
        .route(path::REGISTER_SUCCESS,         get(register_success))
        .route(path::FORGOT_PASSWORD,          get(display_forgot_password).post(post_forgot_password))
        .route(path::FORGOT_PASSWORD_SENT,     get(display_forgot_password_sent))
        .route(path::RESET_PASSWORD_PATTERN,   get(display_reset_password).post(post_reset_password))
}