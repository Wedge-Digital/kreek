use async_trait::async_trait;
use std::fmt;

#[derive(Debug)]
pub enum EmailError {
    Network(String),
    ApiRejected { status: u16, body: String },
}

impl fmt::Display for EmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmailError::Network(msg)                   => write!(f, "Erreur réseau : {}", msg),
            EmailError::ApiRejected { status, body }   => write!(f, "Envoi refusé ({}) : {}", status, body),
        }
    }
}

#[async_trait]
pub trait IEmailService: Send + Sync {
    async fn send(
        &self,
        to:      Vec<String>,
        subject: String,
        html:    String,
    ) -> Result<(), EmailError>;
}