use serde::Deserialize;

pub struct RegisterCommand {
    pub coach_name:       String,
    pub email:            String,
    pub password:         String,
    pub password_confirm: String,
}

#[derive(Deserialize, Debug)]
pub struct PerformLoginCommand {
    pub coach_name: String,
    pub password:   String,
}
