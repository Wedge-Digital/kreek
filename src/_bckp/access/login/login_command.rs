use serde::{Deserialize, Serialize};
use crate::app::infra::middleware::traits::app_message::AppMessage;

#[derive(Serialize, Deserialize)]
pub struct LoginCmd {
    pub username: String,
    pub password: String
}

impl LoginCmd {
    pub fn new(username: String, password: String) -> Self {
        return LoginCmd {
            username,
            password
        };
    }
}

impl AppMessage for LoginCmd {}