use super::error::AuthDomainError;

#[derive(Debug, Clone)]
pub struct CoachName(String);

impl CoachName {
    pub fn new(s: impl Into<String>) -> Result<Self, AuthDomainError> {
        let s = s.into().trim().to_string();
        if s.is_empty() {
            return Err(AuthDomainError::CoachNameEmpty);
        }
        if s.len() > 100 {
            return Err(AuthDomainError::CoachNameTooLong);
        }
        Ok(CoachName(s))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
