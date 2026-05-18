use std::sync::{Arc, Mutex};
use crate::app::auth::context::AuthContext;
use crate::app::spaces::context::SpacesContext;
use crate::app::competitions::context::CompetitionsContext;
use crate::lib::services::email::IEmailService;
use crate::lib::services::event_bus::event_bus::EventBus;

#[derive(Clone)]
pub struct AppState {
    pub auth:             AuthContext,
    pub spaces:           SpacesContext,
    pub competitions:     CompetitionsContext,
    pub email_service:    Arc<dyn IEmailService>,
    pub host_domain:      String,
    pub bypass_auth:      bool,
    pub event_bus:        Arc<Mutex<EventBus>>,
    pub app_event_bus:    Arc<Mutex<EventBus>>,
}