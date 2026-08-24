use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(
        Debug,
        Clone,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        Hash,
        Display,
        AsRef
    )
)]
pub struct NameVo(String);

#[cfg(test)]
mod tests {
    use super::*;

    /// `NameVo` porte quatre alias — `SeasonName`, `MatchDayName`,
    /// `TierName` et le `TeamName` du noyau. Les couvrir ici les couvre tous.
    #[test]
    fn les_noms_admettent_la_typographie_francaise() {
        assert!(NameVo::try_new("Journée d'ouverture".to_string()).is_ok());
        assert!(NameVo::try_new("Saison 2025/2026".to_string()).is_ok());
        assert!(NameVo::try_new("Coupe d’Été — Phase 1".to_string()).is_ok());
        assert!(NameVo::try_new("Élite".to_string()).is_ok());
    }

    #[test]
    fn ce_qui_reste_refuse() {
        assert!(NameVo::try_new("Journée|1".to_string()).is_err());
        assert!(NameVo::try_new("   ".to_string()).is_err());
        assert!(NameVo::try_new("a".repeat(51)).is_err());
    }
}
