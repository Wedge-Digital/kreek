use serde::Deserialize;

// ── Inducement ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Inducement {
    pub uid: String,
    pub name: String,
    pub cost: u32,
    #[serde(rename = "reducedCost")]
    pub reduced_cost: Option<u32>,
    #[serde(rename = "maxQuantity")]
    pub max_quantity: u32,
    pub category: String,
    #[serde(rename = "restrictedTo", default)]
    pub restricted_to: Vec<String>,
    pub description: String,
}

// ── StarPlayer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct StarPlayer {
    pub uid: String,
    pub name: String,
    pub cost: u32,
    #[serde(rename = "MA")]
    pub ma: u8,
    #[serde(rename = "ST")]
    pub st: u8,
    #[serde(rename = "AG")]
    pub ag: String,
    #[serde(rename = "PA")]
    pub pa: String,
    #[serde(rename = "AV")]
    pub av: String,
    #[serde(rename = "playerType")]
    pub player_type: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(rename = "specialAbilityName")]
    pub special_ability_name: String,
    #[serde(rename = "specialAbilityDescription")]
    pub special_ability_description: String,
    #[serde(rename = "playsFor", default)]
    pub plays_for: Vec<String>,
    #[serde(rename = "availableForRosters", default)]
    pub available_for_rosters: Vec<String>,
}

// ── Team ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerPosition {
    pub uid: String,
    #[serde(rename = "positionName")]
    pub position_name: String,
    pub cost: u32,
    #[serde(rename = "MA")]
    pub ma: u8,
    #[serde(rename = "ST")]
    pub st: u8,
    #[serde(rename = "AG")]
    pub ag: u8,
    #[serde(rename = "PA")]
    pub pa: u8,
    #[serde(rename = "AV")]
    pub av: u8,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(rename = "primaryAccess", default)]
    pub primary_access: Vec<String>,
    #[serde(rename = "secondaryAccess", default)]
    pub secondary_access: Vec<String>,
    #[serde(rename = "max_quantity")]
    pub max_quantity: u8,
    #[serde(default)]
    pub is_journeyman: bool,
}

/// Un seul schéma dans le corpus : `{"max": N, "in": [uid, …]}`. Les Élus du
/// Chaos portaient `{"limit", "limitedPlayerIds"}`, aligné par la carte 258 —
/// la struct ne connaît donc plus qu'une forme.
#[derive(Debug, Clone, Deserialize)]
pub struct CrossLimitDefinition {
    pub max: u32,
    #[serde(rename = "in")]
    pub position_uids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Team {
    pub uid: String,
    pub name: String,
    #[serde(rename = "rerollCost")]
    pub reroll_cost: u32,
    pub tier: String,
    #[serde(rename = "specialRules", default)]
    pub special_rules: Vec<String>,
    /// Limites de cumul entre postes — « pas plus de 3 joueurs parmi Ogre,
    /// Troll, Minotaure, Rat Ogre ». Quatre rosters sur trente en ont ; les
    /// autres portent un tableau vide ou pas de champ du tout.
    #[serde(default)]
    pub cross_limit: Vec<CrossLimitDefinition>,
    #[serde(rename = "allowedStaff", default)]
    pub allowed_staff: Vec<String>,
    #[serde(rename = "availablePlayers", default)]
    pub available_players: Vec<PlayerPosition>,
    #[serde(default)]
    pub leagues: Vec<String>,
    #[serde(default)]
    pub logo: Option<String>,
}

// ── SkillCost ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ChosenSkillCost {
    pub primary: u8,
    pub secondary: u8,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct SkillCostLevel {
    pub level: u8,
    pub chosen: ChosenSkillCost,
    #[serde(rename = "chosenElite", default)]
    pub chosen_elite: Option<ChosenSkillCost>,
    pub random: u8,
    #[serde(rename = "randomElite", default)]
    pub random_elite: Option<u8>,
    pub characteristic: u8,
}

impl SkillCostLevel {
    pub fn chosen_for(&self, is_elite: bool) -> &ChosenSkillCost {
        if is_elite {
            self.chosen_elite.as_ref().unwrap_or(&self.chosen)
        } else {
            &self.chosen
        }
    }

    pub fn random_for(&self, is_elite: bool) -> u8 {
        if is_elite {
            self.random_elite.unwrap_or(self.random)
        } else {
            self.random
        }
    }
}

// ── Skill ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Skill {
    pub uid: String,
    pub name: String,
    pub category: String,
    #[serde(rename = "type")]
    pub skill_type: String,
    pub activation: String,
    pub description: String,
}

// ── SkillCategory ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SkillCategory {
    pub id: String,
    pub label: String,
}

// ── SpecialRule ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SpecialRule {
    pub uid: String,
    pub label: String,
}

// ── Barème SPP ────────────────────────────────────────────────────────────────

/// Une ligne de `spp_rules.json` : combien de SPP rapporte un type d'action.
///
/// `action` porte les codes du corpus — `TD`, `CAS`, `REU`, `MVP`, `INT`,
/// `TTM`. `TTM` (passe à un coéquipier) n'a pas encore d'action correspondante
/// dans le jeu : il est chargé et ignoré, plutôt que d'être omis du fichier.
#[derive(Debug, Clone, Deserialize)]
pub struct SppRule {
    pub action: String,
    pub spp: u8,
}

/// Le barème d'une équipe, résolu depuis sa règle spéciale.
///
/// Rendu **entier** plutôt qu'action par action : une seule résolution par
/// joueur, et la garantie qu'un même match ne mélange pas deux barèmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SppScale {
    pub touchdown: u8,
    pub pass: u8,
    pub interception: u8,
    pub casualty: u8,
    pub mvp: u8,
}

/// Le barème par défaut, celui de la très grande majorité des rosters.
///
/// Ces valeurs ne servent que si `spp_rules.json` est illisible ou incomplet —
/// un corpus tiers pourrait ne déclarer aucune table. Elles reproduisent le
/// `normal` du corpus de référence.
impl Default for SppScale {
    fn default() -> Self {
        Self {
            touchdown: 3,
            pass: 1,
            interception: 2,
            casualty: 2,
            mvp: 4,
        }
    }
}

impl SppScale {
    /// Construit le barème depuis les lignes du corpus. Un code d'action absent
    /// garde la valeur par défaut : mieux vaut un barème partiel qu'un panic au
    /// démarrage sur un corpus qu'on ne maîtrise pas.
    pub fn from_rules(rules: &[SppRule]) -> Self {
        let mut scale = Self::default();
        for r in rules {
            match r.action.as_str() {
                "TD" => scale.touchdown = r.spp,
                "REU" => scale.pass = r.spp,
                "INT" => scale.interception = r.spp,
                "CAS" => scale.casualty = r.spp,
                "MVP" => scale.mvp = r.spp,
                _ => {}
            }
        }
        scale
    }
}

// ── Staff ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Staff {
    pub uid: String,
    pub name: String,
    pub price: u32,
    #[serde(rename = "maxQuantity")]
    pub max_quantity: u32,
    pub description: String,
}

// ── League ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct League {
    pub uid: String,
    pub label: String,
}
