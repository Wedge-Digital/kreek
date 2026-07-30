use crate::app::match_report::domain::value_objects::{ActionPlayer, MatchAction, SppCategory};
use crate::app::match_report::ports::{ISppCalculatorPort, PlayerSppDto, SppMatchResult};
use crate::app::references::domain::models::SppScale as RefSppScale;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::spp_calculator::domain::calculator::{self, SppAction, SppActionInput, SppScale};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Le calcul des SPP du récapitulatif — celui qui s'affiche **avant**
/// publication, quand rien n'a encore été crédité à personne.
///
/// Il a besoin du corpus parce que le barème dépend du roster, et c'est à ce
/// titre qu'il porte le dépôt de références : `spp_calculator` reste pur, il
/// reçoit deux barèmes déjà résolus.
pub struct SppCalculatorAdapter {
    reference_repo: Arc<dyn IReferenceRepository>,
}

impl SppCalculatorAdapter {
    pub fn new(reference_repo: Arc<dyn IReferenceRepository>) -> Self {
        Self { reference_repo }
    }

    fn scale_of(&self, roster_id: &str) -> SppScale {
        to_calculator_scale(self.reference_repo.spp_scale_for_roster(roster_id))
    }
}

#[async_trait]
impl ISppCalculatorPort for SppCalculatorAdapter {
    async fn calculate_match_spp(
        &self,
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
        home_roster_id: &str,
        away_roster_id: &str,
    ) -> SppMatchResult {
        let (home_inputs, home_lookup) = to_spp_inputs(home_actions);
        let (away_inputs, away_lookup) = to_spp_inputs(away_actions);
        let result = calculator::calculate(
            &home_inputs,
            &away_inputs,
            self.scale_of(home_roster_id),
            self.scale_of(away_roster_id),
        );
        SppMatchResult {
            home: to_player_spp_dtos(result.home, &home_lookup),
            away: to_player_spp_dtos(result.away, &away_lookup),
        }
    }
}

fn to_calculator_scale(scale: RefSppScale) -> SppScale {
    SppScale {
        touchdown: scale.touchdown,
        pass: scale.pass,
        interception: scale.interception,
        casualty: scale.casualty,
        mvp: scale.mvp,
    }
}

/// La catégorie vient du domaine `match_report`, seul endroit où la
/// correspondance action → SPP soit écrite. Ici on ne fait que la traduire dans
/// le vocabulaire de `spp_calculator`.
fn to_calculator_action(category: SppCategory) -> SppAction {
    match category {
        SppCategory::Touchdown => SppAction::Touchdown,
        SppCategory::Pass => SppAction::Pass,
        SppCategory::Interception => SppAction::Interception,
        SppCategory::Casualty => SppAction::Casualty,
        SppCategory::Mvp => SppAction::Mvp,
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
                action: a.action.spp_category().map(to_calculator_action),
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
