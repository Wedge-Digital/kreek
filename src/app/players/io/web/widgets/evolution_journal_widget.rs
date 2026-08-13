use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{AcquisitionMode, Player, PlayerId, ValueKpo};
use crate::app::players::domain::value_objects::SppCost;
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// ── View models ───────────────────────────────────────────────────────────────

pub struct EvolutionLogRowVm {
    pub label: String,
    pub mode_label: &'static str,
    pub cost: String,
    pub value: String,
    pub origin: &'static str,
}

pub struct EvolutionJournalVm {
    pub player_name: String,
    pub spp_reserve: u32,
    pub evolution_log: Vec<EvolutionLogRowVm>,
    pub can_spend: bool,
    pub spp_spending_widget_url: String,
}

/// Reconstruit le journal directement depuis les events bruts (même principe
/// que `match_history_service::build_match_history`) — c'est le seul moyen de
/// distinguer un bonus de création (`InitialSkillEarned`) d'un achat via les
/// SPP (`PlayerSkillPurchased`) : une fois repliés dans `player.acquired_skills`,
/// les deux events produisent une structure identique, sans champ d'origine.
fn evolution_log_vm(events: &[PlayerDomainEvent]) -> Vec<EvolutionLogRowVm> {
    events.iter().filter_map(evolution_log_row).collect()
}

fn evolution_log_row(event: &PlayerDomainEvent) -> Option<EvolutionLogRowVm> {
    match event {
        PlayerDomainEvent::InitialSkillEarned {
            skill_name,
            mode,
            spp_cost,
            value_delta,
            ..
        } => Some(skill_row(
            skill_name.to_string(),
            *mode,
            *spp_cost,
            *value_delta,
            "Compétence initiale bonus",
        )),
        PlayerDomainEvent::PlayerSkillPurchased {
            skill_name,
            mode,
            spp_cost,
            value_delta,
            ..
        } => Some(skill_row(
            skill_name.to_string(),
            *mode,
            *spp_cost,
            *value_delta,
            "Progression normale",
        )),
        PlayerDomainEvent::PlayerStatIncreased {
            stat,
            spp_cost,
            value_delta,
            ..
        } => Some(EvolutionLogRowVm {
            label: format!("Caractéristique : {}", stat_label(*stat)),
            mode_label: "Choisie",
            cost: format!("{} SPP", spp_cost.into_inner()),
            value: format!("+{} kPo", value_delta.0),
            origin: "Progression normale",
        }),
        _ => None,
    }
}

fn skill_row(
    skill_name: String,
    mode: AcquisitionMode,
    spp_cost: SppCost,
    value_delta: ValueKpo,
    origin: &'static str,
) -> EvolutionLogRowVm {
    let mode_label = match mode {
        AcquisitionMode::Chosen => "Choisie",
        AcquisitionMode::Random => "Aléatoire",
        AcquisitionMode::Customised => "Customisation",
    };
    EvolutionLogRowVm {
        label: skill_name,
        mode_label,
        cost: format!("{} SPP", spp_cost.into_inner()),
        value: format!("+{} kPo", value_delta.0),
        origin,
    }
}

fn stat_label(stat: StatKind) -> &'static str {
    match stat {
        StatKind::Ma => "Mouvement",
        StatKind::St => "Force",
        StatKind::Ag => "Agilité",
        StatKind::Pa => "Passe",
        StatKind::Av => "Armure",
    }
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

#[derive(Deserialize)]
pub struct EvolutionJournalParams {
    #[serde(default)]
    pub can_spend: bool,
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn evolution_journal_widget(
    Path((space_id, player_id)): Path<(String, String)>,
    Query(params): Query<EvolutionJournalParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let player = match load_player(&state, &player_id).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let events = match state
        .players
        .repository
        .find_events_by_id(&PlayerId(player_id.clone()))
        .await
    {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("evolution_journal_widget find_events_by_id {player_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // can_spend est fourni par la page appelante (player_detail_controller, qui
    // a déjà vérifié phase + autorisation) — aucun enjeu de sécurité à le faire
    // transiter tel quel ici : ce widget est en lecture seule, le vrai gardien
    // reste spp_spending_widget (qui revérifie tout indépendamment).
    let vm = EvolutionJournalVm {
        player_name: player.position_name.to_string(),
        spp_reserve: player.spp_remaining(),
        evolution_log: evolution_log_vm(&events),
        can_spend: params.can_spend,
        spp_spending_widget_url: AppRoutes::default()
            .players
            .spp_spending_widget(&space_id, &player_id),
    };

    EvolutionJournalTemplate { vm }.into_response()
}

async fn load_player(state: &AppState, player_id: &str) -> Result<Player, Response> {
    match state
        .players
        .repository
        .find_by_id(&PlayerId(player_id.to_string()))
        .await
    {
        Ok(Some(p)) => Ok(p),
        Ok(None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("evolution_journal_widget find_by_id {player_id}: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::player::TeamId;
    use crate::app::players::domain::value_objects::{SkillId, SkillName};
    use crate::app::shared_kernel::identity::ids::SpaceId;

    fn initial_skill_event() -> PlayerDomainEvent {
        PlayerDomainEvent::InitialSkillEarned {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            skill_id: SkillId::try_new("block".to_string()).unwrap(),
            skill_name: SkillName::try_new("Bloc".to_string()).unwrap(),
            category_css: "type-general".into(),
            mode: AcquisitionMode::Chosen,
            spp_cost: SppCost::try_new(0).unwrap(),
            is_primary: true,
            is_elite: false,
            value_delta: ValueKpo(0),
        }
    }

    fn purchased_skill_event() -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerSkillPurchased {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            skill_id: SkillId::try_new("dodge".to_string()).unwrap(),
            skill_name: SkillName::try_new("Esquive".to_string()).unwrap(),
            category_css: "type-general".into(),
            mode: AcquisitionMode::Chosen,
            spp_cost: SppCost::try_new(6).unwrap(),
            value_delta: ValueKpo(20),
        }
    }

    fn stat_increased_event() -> PlayerDomainEvent {
        PlayerDomainEvent::PlayerStatIncreased {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            stat: StatKind::St,
            spp_cost: SppCost::try_new(14).unwrap(),
            value_delta: ValueKpo(30),
        }
    }

    #[test]
    fn initial_skill_earned_is_labeled_as_bonus_origin() {
        let rows = evolution_log_vm(&[initial_skill_event()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].origin, "Compétence initiale bonus");
        assert_eq!(rows[0].label, "Bloc");
    }

    #[test]
    fn player_skill_purchased_is_labeled_as_normal_progression() {
        let rows = evolution_log_vm(&[purchased_skill_event()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].origin, "Progression normale");
        assert_eq!(rows[0].label, "Esquive");
        assert_eq!(rows[0].value, "+20 kPo");
        assert_eq!(rows[0].cost, "6 SPP");
    }

    #[test]
    fn stat_increase_appears_in_the_journal_with_its_value() {
        let rows = evolution_log_vm(&[stat_increased_event()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Caractéristique : Force");
        assert_eq!(rows[0].origin, "Progression normale");
        assert_eq!(rows[0].value, "+30 kPo");
        assert_eq!(rows[0].cost, "14 SPP");
    }

    #[test]
    fn mixed_events_produce_one_row_each_in_order() {
        let rows = evolution_log_vm(&[
            initial_skill_event(),
            purchased_skill_event(),
            stat_increased_event(),
        ]);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].origin, "Compétence initiale bonus");
        assert_eq!(rows[1].origin, "Progression normale");
        assert_eq!(rows[2].label, "Caractéristique : Force");
    }

    #[test]
    fn unrelated_events_are_ignored() {
        let space_id = SpaceId::new();
        let unrelated = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id,
            position_name: crate::app::players::domain::value_objects::PositionNameVo::try_new(
                "Frappeur".to_string(),
            )
            .unwrap(),
            roster_line_id: crate::app::players::domain::value_objects::RosterLineId::try_new(
                "BLITZER".to_string(),
            )
            .unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: crate::app::players::domain::player::Spp(0),
            starting_value: ValueKpo(100),
        };
        let rows = evolution_log_vm(&[unrelated]);
        assert!(rows.is_empty());
    }
}
