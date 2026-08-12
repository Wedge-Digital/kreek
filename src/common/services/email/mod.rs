pub mod console_email_service;
pub mod email_service;
pub mod resend_mail_service;

pub use console_email_service::ConsoleEmailService;
pub use email_service::{EmailError, IEmailService};
pub use resend_mail_service::ResendMailService;
