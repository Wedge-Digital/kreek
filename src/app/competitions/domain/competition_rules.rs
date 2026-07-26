use crate::app::competitions::domain::error::DomainError;
use crate::app::shared_kernel::tier::{CreationBudget, StartingXp, TierName};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Activated(pub bool);

#[nutype(
    validate(less_or_equal = 100_000),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct RankingPoints(u32);

/// Seuil de TD marqués déclenchant le bonus offensif (≥ seuil).
#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct MinTd(u32);

/// Seuil de TD encaissés en-dessous duquel le bonus défensif s'applique (≤ seuil).
#[nutype(
    validate(less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct MaxTdConceded(u32);

/// Seuil (strict) de sorties infligées déclenchant le bonus agressif (> seuil).
#[nutype(
    validate(less_or_equal = 16),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)
)]
pub struct MinCasualties(u32);

/// Code d'un critère de départage. Validé en forme seulement : le domaine ne
/// connaît pas le catalogue, qui appartient au BC `ranking`. L'appartenance d'un
/// code au catalogue est vérifiée par le use case via `ITiebreakCatalogPort`.
#[nutype(
    validate(not_empty),
    derive(
        Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, AsRef
    )
)]
pub struct TiebreakCode(String);

/// Un critère de départage et son état d'activation. Sa position dans la
/// `TiebreakConfig` porte sa priorité.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TiebreakSetting {
    pub code: TiebreakCode,
    pub activated: Activated,
}

/// Configuration de départage d'une compétition : liste ordonnée de critères,
/// où **l'index porte la priorité**. Une seule source de vérité pour l'ordre —
/// ni priorité en doublon, ni trou de numérotation possibles.
///
/// `#[serde(try_from = ...)]` est indispensable : sans lui, un `Deserialize` nu
/// reconstruirait le newtype sans passer par `try_new`, et n'importe quel payload
/// JSON contournerait les invariants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<TiebreakSetting>")]
pub struct TiebreakConfig(Vec<TiebreakSetting>);

impl TiebreakConfig {
    /// Smart constructor. Refuse une liste vide, une liste sans aucun critère
    /// actif, et tout doublon de code.
    pub fn try_new(settings: Vec<TiebreakSetting>) -> Result<Self, DomainError> {
        if settings.is_empty() {
            return Err(DomainError::EmptyTiebreakConfig);
        }
        Self::ensure_no_duplicate(&settings)?;
        if !settings.iter().any(|s| s.activated.0) {
            return Err(DomainError::NoActiveTiebreaker);
        }
        Ok(Self(settings))
    }

    /// Tous les codes fournis, actifs, dans l'ordre reçu. Les codes viennent du
    /// catalogue : le domaine ne les énumère pas lui-même.
    pub fn all_active(codes: Vec<TiebreakCode>) -> Result<Self, DomainError> {
        let settings = codes
            .into_iter()
            .map(|code| TiebreakSetting {
                code,
                activated: Activated(true),
            })
            .collect();
        Self::try_new(settings)
    }

    /// Lecture ordonnée : l'index de chaque élément **est** sa priorité.
    pub fn settings(&self) -> &[TiebreakSetting] {
        &self.0
    }

    fn ensure_no_duplicate(settings: &[TiebreakSetting]) -> Result<(), DomainError> {
        let mut seen: HashSet<&str> = HashSet::new();
        for setting in settings {
            if !seen.insert(setting.code.as_ref()) {
                return Err(DomainError::DuplicateTiebreakCode {
                    code: setting.code.as_ref().to_string(),
                });
            }
        }
        Ok(())
    }
}

impl TryFrom<Vec<TiebreakSetting>> for TiebreakConfig {
    type Error = DomainError;

    fn try_from(settings: Vec<TiebreakSetting>) -> Result<Self, Self::Error> {
        Self::try_new(settings)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionRules {
    pub ranking_rules: RankingRules,
    pub tiers: Vec<TierRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingRules {
    pub win_points: RankingPoints,
    pub draw_points: RankingPoints,
    pub lose_points: RankingPoints,
    pub offensive_bonus: OffensiveBonus,
    pub defensive_bonus: DefensiveBonus,
    #[serde(default = "default_aggressive_bonus")]
    pub aggressive_bonus: AggressiveBonus,
    pub additionnal_ranking_points: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveBonus {
    pub activated: Activated,
    #[serde(rename = "diff_td")]
    pub min_td: MinTd,
    pub points: RankingPoints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefensiveBonus {
    pub activated: Activated,
    pub points: RankingPoints,
    #[serde(default = "default_max_td_conceded")]
    pub max_td_conceded: MaxTdConceded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggressiveBonus {
    pub activated: Activated,
    pub points: RankingPoints,
    pub min_casualties: MinCasualties,
}

fn default_max_td_conceded() -> MaxTdConceded {
    MaxTdConceded::try_new(1).expect("1 est dans les bornes de MaxTdConceded")
}

fn default_aggressive_bonus() -> AggressiveBonus {
    AggressiveBonus {
        activated: Activated(false),
        points: RankingPoints::try_new(1).expect("1 est dans les bornes de RankingPoints"),
        min_casualties: MinCasualties::try_new(2).expect("2 est dans les bornes de MinCasualties"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierRule {
    pub name: TierName,
    pub budget: CreationBudget,
    pub starting_xp: StartingXp,
    pub rosters: Vec<String>,
    pub inducements: Vec<String>,
    pub star_players: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(raw: &str) -> TiebreakCode {
        TiebreakCode::try_new(raw).expect("code non vide")
    }

    fn setting(raw: &str, activated: bool) -> TiebreakSetting {
        TiebreakSetting {
            code: code(raw),
            activated: Activated(activated),
        }
    }

    #[test]
    fn tiebreak_code_rejects_the_empty_string() {
        assert!(TiebreakCode::try_new("").is_err());
        assert!(TiebreakCode::try_new("nb_td").is_ok());
    }

    #[test]
    fn try_new_rejects_an_empty_configuration() {
        assert_eq!(
            TiebreakConfig::try_new(vec![]),
            Err(DomainError::EmptyTiebreakConfig)
        );
    }

    #[test]
    fn try_new_rejects_a_configuration_without_any_active_criterion() {
        let config = TiebreakConfig::try_new(vec![setting("nb_td", false), setting("nb_cas", false)]);
        assert_eq!(config, Err(DomainError::NoActiveTiebreaker));
    }

    #[test]
    fn try_new_rejects_a_duplicated_code_and_names_it() {
        let config = TiebreakConfig::try_new(vec![
            setting("nb_td", true),
            setting("nb_cas", true),
            setting("nb_td", false),
        ]);
        assert_eq!(
            config,
            Err(DomainError::DuplicateTiebreakCode {
                code: "nb_td".to_string()
            })
        );
    }

    #[test]
    fn try_new_accepts_a_valid_configuration_and_preserves_the_received_order() {
        let config = TiebreakConfig::try_new(vec![
            setting("nb_cas", true),
            setting("nb_td", false),
            setting("diff_td", true),
        ])
        .expect("configuration valide");

        let codes: Vec<&str> = config.settings().iter().map(|s| s.code.as_ref()).collect();
        assert_eq!(codes, vec!["nb_cas", "nb_td", "diff_td"]);
        assert!(!config.settings()[1].activated.0);
    }

    #[test]
    fn all_active_activates_every_code_in_the_received_order() {
        let config = TiebreakConfig::all_active(vec![code("diff_td"), code("nb_td")])
            .expect("liste non vide");

        assert_eq!(config.settings().len(), 2);
        assert!(config.settings().iter().all(|s| s.activated.0));
        assert_eq!(config.settings()[0].code.as_ref(), "diff_td");
    }

    #[test]
    fn all_active_rejects_an_empty_code_list() {
        assert_eq!(
            TiebreakConfig::all_active(vec![]),
            Err(DomainError::EmptyTiebreakConfig)
        );
    }

    #[test]
    fn deserializing_a_valid_array_preserves_order_and_activation() {
        let json = r#"[
            { "code": "diff_td", "activated": true  },
            { "code": "nb_td",   "activated": false }
        ]"#;

        let config: TiebreakConfig = serde_json::from_str(json).expect("tableau valide");

        assert_eq!(config.settings()[0].code.as_ref(), "diff_td");
        assert!(config.settings()[0].activated.0);
        assert!(!config.settings()[1].activated.0);
    }

    #[test]
    fn deserializing_an_array_without_any_active_criterion_fails() {
        // Sans `#[serde(try_from)]`, ce payload contournerait le smart constructor.
        let json = r#"[{ "code": "diff_td", "activated": false }]"#;
        assert!(serde_json::from_str::<TiebreakConfig>(json).is_err());
    }

    #[test]
    fn serializing_produces_a_json_array_in_order() {
        let config = TiebreakConfig::try_new(vec![setting("diff_td", true), setting("nb_td", false)])
            .expect("configuration valide");

        let json = serde_json::to_string(&config).expect("sérialisation");
        assert_eq!(
            json,
            r#"[{"code":"diff_td","activated":true},{"code":"nb_td","activated":false}]"#
        );
    }

    #[test]
    fn serialization_round_trip_is_stable() {
        let config = TiebreakConfig::try_new(vec![setting("nb_cas", true), setting("nb_reu", false)])
            .expect("configuration valide");

        let json = serde_json::to_string(&config).expect("sérialisation");
        let back: TiebreakConfig = serde_json::from_str(&json).expect("désérialisation");
        assert_eq!(back, config);
    }

    #[test]
    fn td_thresholds_accept_valid_and_reject_out_of_bounds() {
        assert!(MinTd::try_new(1).is_ok());
        assert!(MinTd::try_new(16).is_ok());
        assert!(MinTd::try_new(0).is_err());
        assert!(MinTd::try_new(17).is_err());

        assert!(MaxTdConceded::try_new(0).is_ok());
        assert!(MaxTdConceded::try_new(16).is_ok());
        assert!(MaxTdConceded::try_new(17).is_err());

        assert!(MinCasualties::try_new(0).is_ok());
        assert!(MinCasualties::try_new(16).is_ok());
        assert!(MinCasualties::try_new(17).is_err());
    }

    #[test]
    fn legacy_rules_without_new_fields_deserialize_with_defaults() {
        // JSON antérieur à la feature : pas de max_td_conceded ni aggressive_bonus.
        let json = r#"{
            "win_points": 3, "draw_points": 1, "lose_points": 0,
            "offensive_bonus": { "activated": true, "diff_td": 3, "points": 1 },
            "defensive_bonus": { "activated": true, "points": 2 },
            "additionnal_ranking_points": {}
        }"#;

        let rr: RankingRules = serde_json::from_str(json).unwrap();

        assert_eq!(rr.defensive_bonus.max_td_conceded, default_max_td_conceded());
        assert_eq!(rr.aggressive_bonus.activated, Activated(false));
        assert_eq!(rr.aggressive_bonus.min_casualties, MinCasualties::try_new(2).unwrap());
    }

    #[test]
    fn offensive_bonus_keeps_diff_td_json_key_for_min_td_field() {
        let json = r#"{ "activated": true, "diff_td": 5, "points": 1 }"#;
        let ob: OffensiveBonus = serde_json::from_str(json).unwrap();
        assert_eq!(ob.min_td, MinTd::try_new(5).unwrap());

        // Round-trip : la clé JSON reste "diff_td".
        let serialized = serde_json::to_string(&ob).unwrap();
        assert!(serialized.contains("\"diff_td\":5"));
        assert!(!serialized.contains("min_td"));
    }
}
