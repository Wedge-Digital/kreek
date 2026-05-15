use nutype::nutype;
use crate::app::auth::domain::error::AuthDomainError;

#[nutype(
    sanitize(trim, lowercase),
    validate(
        len_char_max = 255,
        regex = r"^[^@\s]+@[^@\s]+\.[^@\s]+$"
    ),
    derive(Debug, Clone, Serialize, Deserialize, AsRef)
)]
pub struct Email(String);

impl Email {
    pub fn value(&self) -> &str {
        self.as_ref()
    }
}

impl From<EmailError> for AuthDomainError {
    fn from(e: EmailError) -> Self {
        match e {
            EmailError::LenCharMaxViolated => AuthDomainError::EmailTooLong,
            EmailError::RegexViolated      => AuthDomainError::EmailInvalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_email() {
        assert!(Email::try_new("bagouze@example.com").is_ok());
    }

    #[test]
    fn sanitize_trims_and_lowercases() {
        let email = Email::try_new("  Bagouze@Example.COM  ").unwrap();
        assert_eq!(email.value(), "bagouze@example.com");
    }

    #[test]
    fn missing_at_is_rejected() {
        assert_eq!(Email::try_new("bagouzeexample.com").unwrap_err(), EmailError::RegexViolated);
    }

    #[test]
    fn missing_domain_dot_is_rejected() {
        assert_eq!(Email::try_new("bagouze@example").unwrap_err(), EmailError::RegexViolated);
    }

    #[test]
    fn empty_local_part_is_rejected() {
        assert_eq!(Email::try_new("@example.com").unwrap_err(), EmailError::RegexViolated);
    }

    #[test]
    fn empty_domain_is_rejected() {
        assert_eq!(Email::try_new("bagouze@").unwrap_err(), EmailError::RegexViolated);
    }

    #[test]
    fn email_exceeding_255_chars_is_rejected() {
        let local = "a".repeat(250);
        let email = format!("{}@b.com", local);
        assert_eq!(Email::try_new(email).unwrap_err(), EmailError::LenCharMaxViolated);
    }

    #[test]
    fn whitespace_in_address_is_rejected() {
        assert_eq!(Email::try_new("ba gouze@example.com").unwrap_err(), EmailError::RegexViolated);
    }
}