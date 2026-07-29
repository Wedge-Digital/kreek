use crate::app::shared_kernel::bloodbowl::ids::RosterId;
use crate::app::shared_kernel::bloodbowl::tier::{CreationBudget, StartingXp, TierName};
use crate::app::team_creation::domain::ruleset::{
    RosterTier, Ruleset, RulesetId, RulesetName, TierId,
};
use serde::{Deserialize, Serialize};

/// Règles de création copiées depuis le contexte Compétition au moment de la création d'équipe.
/// Ce VO appartient au contexte team_creation — jamais de jointure vers competition_seasons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationTier {
    pub name: TierName,
    pub budget: CreationBudget,
    pub start_xp: StartingXp,
    pub rosters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreationRules {
    pub tiers: Vec<CreationTier>,
}

impl CreationRules {
    /// Retourne les SPP de départ pour un roster donné (0 si non trouvé).
    pub fn get_starting_xp_for_roster(&self, roster_uid: &str) -> u8 {
        self.tiers
            .iter()
            .find(|t| t.rosters.iter().any(|r| r == roster_uid))
            .map(|t| t.start_xp.into_inner().min(255) as u8)
            .unwrap_or(0)
    }

    /// Convertit les règles dénormalisées en `Ruleset` domaine,
    /// utilisé pour les validations métier (budget, roster autorisé).
    /// L'identifiant de la saison sert d'id de ruleset — les deux contextes
    /// restent ainsi découplés.
    pub fn to_ruleset(&self, season_id: &str) -> Ruleset {
        Ruleset {
            id: RulesetId(season_id.to_string()),
            name: RulesetName(season_id.to_string()),
            tiers: self
                .tiers
                .iter()
                .map(|t| RosterTier {
                    id: TierId(t.name.clone().into_inner()),
                    name: t.name.clone(),
                    roster_ids: t.rosters.iter().map(|r| RosterId(r.clone())).collect(),
                    budget: t.budget,
                })
                .collect(),
        }
    }
}
