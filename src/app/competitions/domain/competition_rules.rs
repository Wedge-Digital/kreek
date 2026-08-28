use crate::app::competitions::domain::error::DomainError;
use crate::app::shared_kernel::bloodbowl::tier::{CreationBudget, StartingXp, TierName};
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
        Debug,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Serialize,
        Deserialize,
        Display,
        AsRef
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

/// Les trois règles que l'onglet Paramètres fera respecter (épic E14).
///
/// **Aucune ne consomme `self`** : elles empruntent et rendent une copie
/// modifiée. C'est ce qui permettra au panneau de proposer un aperçu sans
/// engager la modification — et ce que vérifie
/// `les_methodes_n_alterent_pas_l_original`.
impl CompetitionRules {
    /// Le barème de classement est **entièrement rouvert** : c'est le seul des
    /// réglages dont chaque champ se modifie en cours de saison.
    pub fn with_ranking_rules(&self, ranking_rules: RankingRules) -> Self {
        Self {
            ranking_rules,
            tiers: self.tiers.clone(),
        }
    }

    /// Ne rouvre que les **coups de pouce** : nom, budget, expérience de départ
    /// et rosters de chaque tier restent tels qu'à la création.
    ///
    /// **Un refus, pas une correction.** Recopier silencieusement les valeurs
    /// d'origine sur un écart rendrait modifiable par requête forgée ce que
    /// l'écran n'ouvre pas — l'appelant croirait avoir changé un budget, et le
    /// désaccord se découvrirait bien plus tard.
    ///
    /// **Les uid de coups de pouce ne sont pas validés** : le corpus vit hors du
    /// dépôt et peut changer sous les pieds d'une compétition. Refuser un uid
    /// inconnu figerait le réglage le jour où le corpus bouge.
    pub fn with_inducements_from(&self, tiers: Vec<TierRule>) -> Result<Self, DomainError> {
        if tiers.len() != self.tiers.len() {
            return Err(DomainError::TierCountChanged {
                before: self.tiers.len(),
                after: tiers.len(),
            });
        }
        for (avant, recu) in self.tiers.iter().zip(tiers.iter()) {
            Self::refuser_tout_ecart(avant, recu)?;
        }
        Ok(Self {
            ranking_rules: self.ranking_rules.clone(),
            tiers,
        })
    }

    /// Les quatre champs figés, comparés un à un pour que l'erreur porte le nom
    /// de celui qui a bougé — « le champ a changé » ne dirait pas lequel.
    fn refuser_tout_ecart(avant: &TierRule, recu: &TierRule) -> Result<(), DomainError> {
        let ecart = |field: &'static str| DomainError::ImmutableTierField {
            tier: avant.name.as_ref().to_string(),
            field,
        };
        if recu.name.as_ref() != avant.name.as_ref() {
            return Err(ecart("name"));
        }
        if recu.budget.0 != avant.budget.0 {
            return Err(ecart("budget"));
        }
        if recu.starting_xp.into_inner() != avant.starting_xp.into_inner() {
            return Err(ecart("starting_xp"));
        }
        if recu.rosters != avant.rosters {
            return Err(ecart("rosters"));
        }
        Ok(())
    }

    /// Un roster ne peut pas figurer dans deux tiers.
    ///
    /// Déplacée depuis `save_competition_rules.rs` (carte 417) : la règle est
    /// métier, elle n'avait rien à faire dans un use case. Le corps est
    /// **copié** tel quel, seul le type d'erreur change.
    pub fn ensure_roster_unicity(&self) -> Result<(), DomainError> {
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for tier in &self.tiers {
            for roster in &tier.rosters {
                if let Some(prev) = seen.insert(roster.as_str(), tier.name.as_ref()) {
                    return Err(DomainError::RosterInMultipleTiers {
                        roster: roster.clone(),
                        tiers: (prev.to_string(), tier.name.as_ref().to_string()),
                    });
                }
            }
        }
        Ok(())
    }
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
    /// Critères de départage, ordonnés par priorité décroissante. Remplace
    /// l'ancien `additionnal_ranking_points: HashMap<String, u32>`, qui ne
    /// portait que l'ordre et pas l'activation.
    pub tiebreakers: TiebreakConfig,
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
        let config =
            TiebreakConfig::try_new(vec![setting("nb_td", false), setting("nb_cas", false)]);
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
        let config =
            TiebreakConfig::try_new(vec![setting("diff_td", true), setting("nb_td", false)])
                .expect("configuration valide");

        let json = serde_json::to_string(&config).expect("sérialisation");
        assert_eq!(
            json,
            r#"[{"code":"diff_td","activated":true},{"code":"nb_td","activated":false}]"#
        );
    }

    #[test]
    fn serialization_round_trip_is_stable() {
        let config =
            TiebreakConfig::try_new(vec![setting("nb_cas", true), setting("nb_reu", false)])
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
        // JSON antérieur à la feature bonus : pas de max_td_conceded ni
        // aggressive_bonus. `tiebreakers` est en revanche requis — le champ n'a
        // pas de valeur par défaut, le domaine ne connaissant pas le catalogue.
        let json = r#"{
            "win_points": 3, "draw_points": 1, "lose_points": 0,
            "offensive_bonus": { "activated": true, "diff_td": 3, "points": 1 },
            "defensive_bonus": { "activated": true, "points": 2 },
            "tiebreakers": [{ "code": "diff_td", "activated": true }]
        }"#;

        let rr: RankingRules = serde_json::from_str(json).unwrap();

        assert_eq!(
            rr.defensive_bonus.max_td_conceded,
            default_max_td_conceded()
        );
        assert_eq!(rr.aggressive_bonus.activated, Activated(false));
        assert_eq!(
            rr.aggressive_bonus.min_casualties,
            MinCasualties::try_new(2).unwrap()
        );
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

#[cfg(test)]
mod reglages_rouverts_tests {
    //! Les règles que l'onglet Paramètres fera respecter (carte 417).
    use super::*;

    fn tier(nom: &str, budget: u32, xp: u32, rosters: &[&str], coups: &[&str]) -> TierRule {
        TierRule {
            name: TierName::try_new(nom.to_string()).unwrap(),
            budget: CreationBudget(budget),
            starting_xp: StartingXp::try_new(xp).unwrap(),
            rosters: rosters.iter().map(|r| r.to_string()).collect(),
            inducements: coups.iter().map(|c| c.to_string()).collect(),
            star_players: vec![],
        }
    }

    fn regles(tiers: Vec<TierRule>) -> CompetitionRules {
        CompetitionRules {
            ranking_rules: bareme(3),
            tiers,
        }
    }

    fn bareme(victoire: u32) -> RankingRules {
        RankingRules {
            win_points: RankingPoints::try_new(victoire).unwrap(),
            draw_points: RankingPoints::try_new(1).unwrap(),
            lose_points: RankingPoints::try_new(0).unwrap(),
            offensive_bonus: OffensiveBonus {
                activated: Activated(false),
                min_td: MinTd::try_new(2).unwrap(),
                points: RankingPoints::try_new(1).unwrap(),
            },
            defensive_bonus: DefensiveBonus {
                activated: Activated(false),
                points: RankingPoints::try_new(1).unwrap(),
                max_td_conceded: default_max_td_conceded(),
            },
            aggressive_bonus: default_aggressive_bonus(),
            tiebreakers: TiebreakConfig::all_active(vec![code("nb_td")]).unwrap(),
        }
    }

    fn code(raw: &str) -> TiebreakCode {
        TiebreakCode::try_new(raw).unwrap()
    }

    fn deux_tiers() -> Vec<TierRule> {
        vec![
            tier("Élite", 1000, 0, &["HUMAN"], &["BABE"]),
            tier("Amateurs", 1200, 6, &["ORC"], &[]),
        ]
    }

    // ── with_inducements_from ────────────────────────────────────────────────

    #[test]
    fn with_inducements_from_accepte_un_changement_de_coups_de_pouce() {
        let avant = regles(deux_tiers());
        let mut recus = deux_tiers();
        recus[0].inducements = vec!["BABE".into(), "BLOODWEISER".into()];
        recus[1].inducements = vec!["APOTHECARY".into()];

        let apres = avant.with_inducements_from(recus).expect("cas nominal");

        assert_eq!(apres.tiers[0].inducements.len(), 2);
        assert_eq!(apres.tiers[1].inducements, vec!["APOTHECARY".to_string()]);
    }

    #[test]
    fn with_inducements_from_accepte_un_tier_sans_coup_de_pouce() {
        let avant = regles(deux_tiers());
        let mut recus = deux_tiers();
        recus[0].inducements = vec![];

        let apres = avant
            .with_inducements_from(recus)
            .expect("liste vide valide");

        assert!(apres.tiers[0].inducements.is_empty());
    }

    /// Les quatre refus sont écrits séparément et non en boucle : l'erreur porte
    /// le nom du champ, et c'est ce nom qu'on veut voir échouer nommément.
    #[test]
    fn with_inducements_from_refuse_un_budget_modifie() {
        let avant = regles(deux_tiers());
        let mut recus = deux_tiers();
        recus[0].budget = CreationBudget(999);

        match avant.with_inducements_from(recus) {
            Err(DomainError::ImmutableTierField { tier, field }) => {
                assert_eq!((tier.as_str(), field), ("Élite", "budget"));
            }
            autre => panic!("attendu un refus de budget : {autre:?}"),
        }
    }

    #[test]
    fn with_inducements_from_refuse_un_nom_modifie() {
        let avant = regles(deux_tiers());
        let mut recus = deux_tiers();
        recus[1].name = TierName::try_new("Renommé".to_string()).unwrap();

        match avant.with_inducements_from(recus) {
            Err(DomainError::ImmutableTierField { tier, field }) => {
                assert_eq!((tier.as_str(), field), ("Amateurs", "name"));
            }
            autre => panic!("attendu un refus de nom : {autre:?}"),
        }
    }

    #[test]
    fn with_inducements_from_refuse_un_xp_modifie() {
        let avant = regles(deux_tiers());
        let mut recus = deux_tiers();
        recus[1].starting_xp = StartingXp::try_new(42).unwrap();

        match avant.with_inducements_from(recus) {
            Err(DomainError::ImmutableTierField { field, .. }) => {
                assert_eq!(field, "starting_xp");
            }
            autre => panic!("attendu un refus d'expérience : {autre:?}"),
        }
    }

    #[test]
    fn with_inducements_from_refuse_des_rosters_modifies() {
        let avant = regles(deux_tiers());
        let mut recus = deux_tiers();
        recus[0].rosters = vec!["HUMAN".into(), "ELF".into()];

        match avant.with_inducements_from(recus) {
            Err(DomainError::ImmutableTierField { field, .. }) => {
                assert_eq!(field, "rosters");
            }
            autre => panic!("attendu un refus de rosters : {autre:?}"),
        }
    }

    #[test]
    fn with_inducements_from_refuse_un_tier_ajoute() {
        let avant = regles(deux_tiers());
        let mut recus = deux_tiers();
        recus.push(tier("Bonus", 500, 0, &["SKAVEN"], &[]));

        match avant.with_inducements_from(recus) {
            Err(DomainError::TierCountChanged { before, after }) => {
                assert_eq!((before, after), (2, 3));
            }
            autre => panic!("attendu un refus de dénombrement : {autre:?}"),
        }
    }

    #[test]
    fn with_inducements_from_refuse_un_tier_retire() {
        let avant = regles(deux_tiers());
        let recus = vec![deux_tiers().remove(0)];

        match avant.with_inducements_from(recus) {
            Err(DomainError::TierCountChanged { before, after }) => {
                assert_eq!((before, after), (2, 1));
            }
            autre => panic!("attendu un refus de dénombrement : {autre:?}"),
        }
    }

    // ── ensure_roster_unicity — déplacée depuis le use case ──────────────────

    #[test]
    fn ensure_roster_unicity_accepte_des_tiers_disjoints() {
        assert!(regles(deux_tiers()).ensure_roster_unicity().is_ok());
    }

    #[test]
    fn ensure_roster_unicity_refuse_un_roster_dans_deux_tiers() {
        let mut tiers = deux_tiers();
        tiers[1].rosters = vec!["HUMAN".into()];

        match regles(tiers).ensure_roster_unicity() {
            Err(DomainError::RosterInMultipleTiers { roster, tiers }) => {
                assert_eq!(roster, "HUMAN");
                assert_eq!(tiers, ("Élite".to_string(), "Amateurs".to_string()));
            }
            autre => panic!("attendu un refus d'unicité : {autre:?}"),
        }
    }

    // ── Non-consommation ─────────────────────────────────────────────────────

    /// Les trois méthodes empruntent et rendent une copie : l'original reste
    /// intact. C'est ce qui permettra au panneau de proposer un **aperçu** sans
    /// engager la modification — sans quoi il faudrait relire la saison pour
    /// annuler.
    #[test]
    fn les_methodes_n_alterent_pas_l_original() {
        let avant = regles(deux_tiers());

        let mut recus = deux_tiers();
        recus[0].inducements = vec!["BABE".into(), "BLOODWEISER".into()];
        let modifiees = avant.with_inducements_from(recus).unwrap();
        let rebareme = avant.with_ranking_rules(bareme(5));

        assert_eq!(
            avant.tiers[0].inducements,
            vec!["BABE".to_string()],
            "les coups de pouce d'origine ont bougé"
        );
        assert_eq!(
            avant.ranking_rules.win_points.into_inner(),
            3,
            "le barème d'origine a bougé"
        );
        // Et les copies, elles, portent bien la modification.
        assert_eq!(modifiees.tiers[0].inducements.len(), 2);
        assert_eq!(rebareme.ranking_rules.win_points.into_inner(), 5);
    }

    #[test]
    fn with_ranking_rules_ne_touche_pas_aux_tiers() {
        let avant = regles(deux_tiers());

        let apres = avant.with_ranking_rules(bareme(5));

        assert_eq!(apres.tiers.len(), 2);
        assert_eq!(apres.tiers[0].inducements, vec!["BABE".to_string()]);
    }
}
