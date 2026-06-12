use crate::common::services::email::{EmailError, IEmailService};
use async_trait::async_trait;

pub struct ConsoleEmailService;

#[async_trait]
impl IEmailService for ConsoleEmailService {
    async fn send(&self, to: Vec<String>, subject: String, html: String) -> Result<(), EmailError> {
        println!("┌─ Email ─────────────────────────────────────────");
        println!("│ To      : {}", to.join(", "));
        println!("│ Subject : {}", subject);
        println!("├─────────────────────────────────────────────────");
        println!("{}", html);
        println!("└─────────────────────────────────────────────────");
        Ok(())
    }
}
