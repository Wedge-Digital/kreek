use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;
use std::fmt::Display;

#[nutype(
    sanitize(trim),
    validate(
        len_char_max = 100,
        regex = TEXTE_SAISI
    ),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef)
)]
pub struct CompetitionName(String);

impl CompetitionName {
    pub fn value(&self) -> &str {
        self.as_ref()
    }
}

impl Display for CompetitionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_basic_name() {
        assert!(CompetitionName::try_new("Ligue des Champions").is_ok());
    }

    /// Le tiret cadratin était refusé, et le test gravait ce refus au lieu de
    /// le contester. C'est pourtant la ponctuation qu'un traitement de texte
    /// produit tout seul.
    #[test]
    fn valid_with_accents() {
        assert!(CompetitionName::try_new("Championnat Étoilé — Saison 5").is_ok());
        assert!(CompetitionName::try_new("Championnat Etoile Saison 5").is_ok());
    }

    #[test]
    fn valid_with_punctuation() {
        assert!(CompetitionName::try_new("Ligue (Hiver) 2026 - Phase 1").is_ok());
    }

    #[test]
    fn valid_with_apostrophe() {
        assert!(CompetitionName::try_new("L'Académie des Coachs").is_ok());
    }

    #[test]
    fn trims_whitespace() {
        let name = CompetitionName::try_new("  Ma Ligue  ").unwrap();
        assert_eq!(name.value(), "Ma Ligue");
    }

    #[test]
    fn rejects_too_long() {
        let name = "a".repeat(101);
        assert!(CompetitionName::try_new(name).is_err());
    }

    #[test]
    fn accepts_exactly_100_chars() {
        let name = "a".repeat(100);
        assert!(CompetitionName::try_new(name).is_ok());
    }

    #[test]
    fn rejects_invalid_chars() {
        assert!(CompetitionName::try_new("Ligue|Interdite").is_err());
    }
}
