use serde::{Deserialize, Serialize};
use crate::app::infra::middleware::traits::app_message::AppMessage;

#[derive(Serialize, Deserialize, Clone)]
pub struct RegisterCmd {
    pub username: String,
    pub password: String,
    pub email: String,
    pub firstname: String,
    pub lastname: String,
    pub coach_name: String
}

impl AppMessage for RegisterCmd {}