use std::collections::HashSet;

/// Entrée opaque du calcul SPP — ne dépend d'aucun type du domaine `match_report`.
/// `actor_key` est fourni par l'appelant (BC match_report, via son adapter) et sert
/// uniquement de clé de regroupement, sans signification pour `spp_calculator`.
pub struct SppActionInput {
    pub actor_key: String,
    pub is_injury: bool,
}

pub struct SppCalculationResult {
    pub home: Vec<(String, u8)>,
    pub away: Vec<(String, u8)>,
}

/// STUB — retourne une valeur plausible (10 SPP) par acteur distinct ayant au moins
/// une action non subie (home et away traités indépendamment). La vraie règle de calcul
/// (quelles actions donnent combien de SPP, sélection de ruleset Normal/Brutal) est hors
/// scope de cette carte — carte dédiée future.
pub fn calculate(home_actions: &[SppActionInput], away_actions: &[SppActionInput]) -> SppCalculationResult {
    SppCalculationResult {
        home: distinct_non_injury_actors(home_actions),
        away: distinct_non_injury_actors(away_actions),
    }
}

const STUB_SPP: u8 = 10;

fn distinct_non_injury_actors(actions: &[SppActionInput]) -> Vec<(String, u8)> {
    let mut seen = HashSet::new();
    let mut result = vec![];
    for action in actions {
        if action.is_injury {
            continue;
        }
        if seen.insert(action.actor_key.clone()) {
            result.push((action.actor_key.clone(), STUB_SPP));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_stub_never_credits_spp_to_an_injury_only_actor() {
        let actions = vec![SppActionInput { actor_key: "p1".to_string(), is_injury: true }];
        let result = calculate(&actions, &[]);
        assert!(result.home.is_empty());
    }

    #[test]
    fn calculate_stub_credits_flat_spp_to_other_actors() {
        let actions = vec![
            SppActionInput { actor_key: "p1".to_string(), is_injury: false },
            SppActionInput { actor_key: "p1".to_string(), is_injury: true },
            SppActionInput { actor_key: "p2".to_string(), is_injury: false },
        ];
        let result = calculate(&actions, &[]);
        assert_eq!(result.home.len(), 2);
        assert!(result.home.contains(&("p1".to_string(), 10)));
        assert!(result.home.contains(&("p2".to_string(), 10)));
    }
}
