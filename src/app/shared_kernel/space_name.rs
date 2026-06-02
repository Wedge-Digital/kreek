use nutype::nutype;
use std::fmt::Display;

#[nutype(
    sanitize(trim),
    validate(len_char_max = 100, regex = r"^[\p{L}0-9_\-' ]+$"),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef)
)]
pub struct SpaceName(String);

impl SpaceName {
    pub fn value(&self) -> &str {
        self.as_ref()
    }
}

impl Display for SpaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_name() {
        assert!(SpaceName::try_new("my_space-01").is_ok());
    }

    #[test]
    fn valid_letters_only() {
        assert!(SpaceName::try_new("MySpace").is_ok());
    }

    #[test]
    fn trims_whitespace() {
        let name = SpaceName::try_new("  my_space  ").unwrap();
        assert_eq!(name.value(), "my_space");
    }

    #[test]
    fn valid_with_spaces() {
        assert!(SpaceName::try_new("Ligue des Zazous").is_ok());
    }

    #[test]
    fn valid_with_apostrophe() {
        assert!(SpaceName::try_new("L'Académie").is_ok());
    }

    #[test]
    fn valid_with_spaces_and_apostrophe() {
        assert!(SpaceName::try_new("L'Espace des Champions").is_ok());
    }

    #[test]
    fn valid_accented_chars() {
        assert!(SpaceName::try_new("Équipe Étoilée").is_ok());
    }

    #[test]
    fn rejects_special_chars() {
        assert_eq!(
            SpaceName::try_new("my@space!").unwrap_err(),
            SpaceNameError::RegexViolated
        );
    }

    #[test]
    fn rejects_empty_after_trim() {
        assert_eq!(
            SpaceName::try_new("   ").unwrap_err(),
            SpaceNameError::RegexViolated
        );
    }

    #[test]
    fn rejects_name_exceeding_100_chars() {
        let name = "a".repeat(101);
        assert_eq!(
            SpaceName::try_new(name).unwrap_err(),
            SpaceNameError::LenCharMaxViolated
        );
    }

    #[test]
    fn accepts_name_of_exactly_100_chars() {
        let name = "a".repeat(100);
        assert!(SpaceName::try_new(name).is_ok());
    }
}
