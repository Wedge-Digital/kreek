use crate::app::players::domain::player::{Player, PlayerId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────
// Copiés tels quels depuis l'ancien bloc inline de player_detail_controller.rs
// (carte 181 — extraction en widget autonome pour le slot de la fiche joueur).

pub struct EvolutionLogRowVm {
    pub label:      String,
    pub mode_label: &'static str,
    pub cost:       String,
    pub value:      String,
    pub origin:     &'static str,
}

pub struct EvolutionJournalVm {
    pub player_name: String,
    pub spp_reserve: u32,
    pub evolution_log: Vec<EvolutionLogRowVm>,
}

fn evolution_log_vm(player: &Player) -> Vec<EvolutionLogRowVm> {
    player.acquired_skills.iter().map(|s| {
        let mode_label = match s.mode {
            crate::app::players::domain::player::AcquisitionMode::Chosen => "Choisie",
            crate::app::players::domain::player::AcquisitionMode::Random => "Aléatoire",
        };
        EvolutionLogRowVm {
            label:  s.skill_name.to_string(),
            mode_label,
            cost:   format!("{} SPP", s.spp_cost.into_inner()),
            value:  String::new(),
            origin: "Compétence initiale bonus",
        }
    }).collect()
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "evolution-journal-widget.html")]
pub struct EvolutionJournalTemplate {
    pub vm: EvolutionJournalVm,
}

impl IntoResponse for EvolutionJournalTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("evolution_journal_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn evolution_journal_widget(
    Path((_space_id, player_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let player = match state.players.repository.find_by_id(&PlayerId(player_id.clone())).await {
        Ok(Some(p)) => p,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("evolution_journal_widget find_by_id {player_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let vm = EvolutionJournalVm {
        player_name: player.position_name.to_string(),
        spp_reserve: player.spp_remaining(),
        evolution_log: evolution_log_vm(&player),
    };

    EvolutionJournalTemplate { vm }.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::player::{AcquiredSkill, AcquisitionMode, Spp, TeamId, ValueKpo};
    use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId, SkillId, SkillName, SppCost};
    use crate::app::shared_kernel::common_types::SpaceId;

    fn sample_player_with_spp(spp_earned: u32, skill_costs: &[u8]) -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()), team_id: TeamId("t1".into()), space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None, base_skills: vec![], starting_spp: Spp(0), starting_value: ValueKpo(100_000),
        };
        let mut player = Player::from_events(&[created]).unwrap();
        player.spp = Spp(spp_earned);
        for (i, cost) in skill_costs.iter().enumerate() {
            player.acquired_skills.push(AcquiredSkill {
                skill_id: SkillId::try_new(format!("s{i}")).unwrap(),
                skill_name: SkillName::try_new(format!("Skill{i}")).unwrap(),
                mode: AcquisitionMode::Chosen,
                spp_cost: SppCost::try_new(*cost).unwrap(),
                value_delta: ValueKpo(0),
            });
        }
        player
    }

    #[test]
    fn evolution_log_vm_labels_acquired_skills_as_bonus_origin() {
        let player = sample_player_with_spp(10, &[6]);
        let rows = evolution_log_vm(&player);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].origin, "Compétence initiale bonus");
        assert_eq!(rows[0].cost, "6 SPP");
    }
}
