use crate::app::auth::domain::error::AuthDomainError;
use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = r"^[\p{L}0-9._ -]+$"),
    derive(Eq, Hash, PartialEq, Debug, Clone, Serialize, Deserialize, Display)
)]
pub struct CoachName(String);

impl From<CoachNameError> for AuthDomainError {
    fn from(e: CoachNameError) -> Self {
        match e {
            CoachNameError::NotEmptyViolated => AuthDomainError::CoachNameEmpty,
            CoachNameError::LenCharMaxViolated => AuthDomainError::CoachNameTooLong,
            CoachNameError::RegexViolated => AuthDomainError::CoachNameInvalidChars,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_simple_name() {
        assert!(CoachName::try_new("Bagouze").is_ok());
    }

    #[test]
    fn valid_with_spaces() {
        assert!(CoachName::try_new("Dark Nagash").is_ok());
    }

    #[test]
    fn valid_with_digits() {
        assert!(CoachName::try_new("Coach42").is_ok());
    }

    #[test]
    fn valid_exactly_50_chars() {
        let name = "a".repeat(50);
        assert!(CoachName::try_new(name).is_ok());
    }

    #[test]
    fn sanitize_trims_leading_trailing_spaces() {
        let coach = CoachName::try_new("  Bagouze  ").unwrap();
        assert_eq!(coach.into_inner(), "Bagouze");
    }

    #[test]
    fn empty_string_is_rejected() {
        assert_eq!(
            CoachName::try_new("").unwrap_err(),
            CoachNameError::NotEmptyViolated,
        );
    }

    #[test]
    fn whitespace_only_is_rejected_after_trim() {
        assert_eq!(
            CoachName::try_new("   ").unwrap_err(),
            CoachNameError::NotEmptyViolated,
        );
    }

    #[test]
    fn name_exceeding_50_chars_is_rejected() {
        let name = "a".repeat(51);
        assert_eq!(
            CoachName::try_new(name).unwrap_err(),
            CoachNameError::LenCharMaxViolated,
        );
    }

    #[test]
    fn special_characters_are_rejected() {
        assert_eq!(
            CoachName::try_new("Bag@uze").unwrap_err(),
            CoachNameError::RegexViolated,
        );
    }

    #[test]
    fn accented_characters_are_valid() {
        assert!(CoachName::try_new("Bâgouze").is_ok());
        assert!(CoachName::try_new("Hervé").is_ok());
        assert!(CoachName::try_new("Ñoño").is_ok());
    }

    #[test]
    fn hyphen_is_valid() {
        assert!(CoachName::try_new("Dark-Nagash").is_ok());
    }

    #[test]
    fn underscore_is_valid() {
        assert!(CoachName::try_new("Bagouze_2").is_ok());
    }

    #[test]
    fn special_characters_are_still_rejected() {
        for name in ["Bag@uze", "Coach!", "Test#1", "foo/bar"] {
            assert_eq!(
                CoachName::try_new(name).unwrap_err(),
                CoachNameError::RegexViolated,
                "'{name}' aurait dû être rejeté",
            );
        }
    }
}
