use crate::app::match_report::domain::value_objects::{ActionPlayer, MatchAction, MatchActionType};
use crate::app::match_report::ports::{ISppCalculatorPort, PlayerSppDto, SppMatchResult};
use crate::app::spp_calculator::domain::calculator::{self, SppActionInput};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct SppCalculatorAdapter;

#[async_trait]
impl ISppCalculatorPort for SppCalculatorAdapter {
    async fn calculate_match_spp(
        &self,
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
        _home_roster_id: &str,
        _away_roster_id: &str,
    ) -> SppMatchResult {
        let (home_inputs, home_lookup) = to_spp_inputs(home_actions);
        let (away_inputs, away_lookup) = to_spp_inputs(away_actions);
        let result = calculator::calculate(&home_inputs, &away_inputs);
        SppMatchResult {
            home: to_player_spp_dtos(result.home, &home_lookup),
            away: to_player_spp_dtos(result.away, &away_lookup),
        }
    }
}

fn actor_key(player: &ActionPlayer) -> String {
    match player {
        ActionPlayer::Regular(player_id) => player_id.to_string(),
        ActionPlayer::Temp(temp_id) => temp_id.0.clone(),
    }
}

fn to_spp_inputs(actions: &[MatchAction]) -> (Vec<SppActionInput>, HashMap<String, ActionPlayer>) {
    let mut lookup = HashMap::new();
    let inputs = actions
        .iter()
        .map(|a| {
            let key = actor_key(&a.player);
            lookup
                .entry(key.clone())
                .or_insert_with(|| a.player.clone());
            SppActionInput {
                actor_key: key,
                is_injury: matches!(a.action, MatchActionType::Blesse { .. }),
            }
        })
        .collect();
    (inputs, lookup)
}

fn to_player_spp_dtos(
    entries: Vec<(String, u8)>,
    lookup: &HashMap<String, ActionPlayer>,
) -> Vec<PlayerSppDto> {
    entries
        .into_iter()
        .filter_map(|(key, spp)| {
            lookup.get(&key).map(|player| PlayerSppDto {
                action_player: player.clone(),
                spp,
            })
        })
        .collect()
}
