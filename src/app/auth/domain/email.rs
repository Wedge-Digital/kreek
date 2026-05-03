use super::error::AuthDomainError;

#[derive(Debug, Clone)]
pub struct Email(String);

impl Email {
    pub fn new(s: impl Into<String>) -> Result<Self, AuthDomainError> {
        let s = s.into().trim().to_lowercase();
        if s.len() > 255 {
            return Err(AuthDomainError::EmailTooLong);
        }
        let at = s.find('@').ok_or(AuthDomainError::EmailInvalid)?;
        let domain = &s[at + 1..];
        if domain.is_empty() || !domain.contains('.') {
            return Err(AuthDomainError::EmailInvalid);
        }
        Ok(Email(s))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
