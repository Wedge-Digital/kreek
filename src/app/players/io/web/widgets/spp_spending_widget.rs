use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{Player, PlayerId};
use crate::app::players::io::web::purchase_skill_controller::can_spend_spp;
use crate::app::players::io::web::widgets::evolution_journal_widget::evolution_journal_widget;
use crate::app::players::ports::ISkillCatalogPort;
use crate::app::players::use_cases::improvement_cost_service::resolve_stat_cost;
use crate::app::players::use_cases::player_stats_service::{resolve_stats, ResolvedPlayerStats};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────

pub struct StatCardVm {
    pub label:           &'static str,
    pub current_display: String,
    pub next_display:    String,
    pub cost:             u8,
    pub value_delta_kpo:  u32,
    pub can_afford:       bool,
    pub increase_url:     String,
}

pub struct SppSpendingVm {
    pub spp_remaining:      u32,
    pub skill_picker_url:   String,
    pub purchase_skill_url: String,
    pub stat_cards:         Vec<StatCardVm>,
}

/// `players::ValueKpo` (retourné par le port catalogue) exprime en réalité des
/// Po bruts, jamais des kPo — division nécessaire pour un affichage cohérent
/// avec la table officielle (+20 kPo, +40 kPo, …). Voir doc du port.
fn to_kpo(raw_po: u32) -> u32 {
    raw_po / 1000
}

fn stat_defs() -> [(StatKind, &'static str, &'static str, bool); 5] {
    // (stat, segment d'URL — cf. parse_stat(), label affiché, cible à atteindre (affichage "+"))
    [
        (StatKind::Ma, "ma", "MA", false),
        (StatKind::St, "st", "ST", false),
        (StatKind::Ag, "ag", "AG", true),
        (StatKind::Pa, "pa", "PA", true),
        (StatKind::Av, "av", "AV", true),
    ]
}

fn display_stat(value: u8, is_target: bool) -> String {
    if is_target { format!("{value}+") } else { value.to_string() }
}

fn next_stat_value(stat: StatKind, current: u8) -> u8 {
    match stat {
        StatKind::Ma | StatKind::St | StatKind::Av => current.saturating_add(1),
        StatKind::Ag | StatKind::Pa => current.saturating_sub(1),
    }
}

fn build_stat_cards(
    stats: ResolvedPlayerStats,
    spp_remaining: u32,
    level: u8,
    catalog: &dyn ISkillCatalogPort,
    routes: &AppRoutes,
    space_id: &str,
    player_id: &str,
) -> Vec<StatCardVm> {
    stat_defs().into_iter().map(|(stat, segment, label, is_target)| {
        let current = match stat {
            StatKind::Ma => stats.ma, StatKind::St => stats.st, StatKind::Ag => stats.ag,
            StatKind::Pa => stats.pa, StatKind::Av => stats.av,
        };
        let (cost, value_delta) = resolve_stat_cost(catalog, stat, level);
        let cost = cost.into_inner();
        StatCardVm {
            label,
            current_display: display_stat(current, is_target),
            next_display:     display_stat(next_stat_value(stat, current), is_target),
            cost,
            value_delta_kpo: to_kpo(value_delta.0),
            can_afford:      spp_remaining >= cost as u32,
            increase_url:    routes.players.increase_stat(space_id, player_id, segment),
        }
    }).collect()
}

fn build_vm(
    player: &Player,
    routes: &AppRoutes,
    space_id: &str,
    catalog: &dyn ISkillCatalogPort,
) -> Option<SppSpendingVm> {
    let stats = resolve_stats(player, catalog)?;
    let level = player.next_improvement_level();
    let spp_remaining = player.spp_remaining();
    let player_id = player.id.0.as_str();

    let acquired_csv = player.acquired_skills.iter()
        .map(|s| s.skill_id.as_ref().to_string())
        .collect::<Vec<_>>()
        .join(",");

    let skill_picker_url = format!(
        "{}?roster_line_id={}&spp_remaining={}&acquired={}&level={}",
        routes.references.skill_picker_base(),
        player.roster_line_id.as_ref(),
        spp_remaining,
        acquired_csv,
        level,
    );

    Some(SppSpendingVm {
        spp_remaining,
        skill_picker_url,
        purchase_skill_url: routes.players.purchase_skill(space_id, player_id),
        stat_cards: build_stat_cards(stats, spp_remaining, level, catalog, routes, space_id, player_id),
    })
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "spp-spending-panel.html")]
pub struct SppSpendingTemplate {
    pub vm: SppSpendingVm,
}

impl IntoResponse for SppSpendingTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("spp_spending_widget render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn spp_spending_widget(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user.clone() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let player = match state.players.repository.find_by_id(&PlayerId(player_id.clone())).await {
        Ok(Some(p)) => p,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("spp_spending_widget find_by_id {player_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if !is_eligible(&state, &user, &space_id, &player).await {
        // Défense en profondeur : si l'URL est atteinte directement hors
        // contexte éligible, on retombe sur le journal en lecture seule.
        let params = crate::app::players::io::web::widgets::evolution_journal_widget::EvolutionJournalParams { can_spend: false };
        return evolution_journal_widget(Path((space_id, player_id)), axum::extract::Query(params), State(state)).await.into_response();
    }

    let app_routes = AppRoutes::default();
    let catalog = state.players.skill_catalog.as_ref();

    match build_vm(&player, &app_routes, &space_id, catalog) {
        Some(vm) => SppSpendingTemplate { vm }.into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn is_eligible(state: &AppState, user: &crate::app::auth::domain::user::User, space_id: &str, player: &Player) -> bool {
    let Some(team) = state.players.roster_port.find_team_info(&player.team_id.0).await else { return false; };
    let Ok(space_id_vo) = SpaceId::try_new(space_id) else { return false; };
    team.in_player_improvement_phase && can_spend_spp(state, user, &space_id_vo, &team).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::player::{AcquiredSkill, AcquisitionMode, Spp, TeamId, ValueKpo};
    use crate::app::players::domain::value_objects::{PositionNameVo, RosterLineId, SkillId, SkillName, SppCost};
    use crate::app::references::io::repository::in_memory_reference_repository::InMemoryReferenceRepository;
    use crate::app::shared_kernel::identity::ids::SpaceId;
    use crate::infrastructure::players::skill_catalog_adapter::SkillCatalogAdapter;

    fn sample_player() -> Player {
        let created = PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()), team_id: TeamId("t1".into()), space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Piétaille des Carrières".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("DEMO_GRANIT__PIETAILLE".to_string()).unwrap(),
            jersey: None, base_skills: vec![], starting_spp: Spp(30), starting_value: ValueKpo(50_000),
        };
        Player::from_events(&[created]).unwrap()
    }

    #[test]
    fn display_stat_appends_plus_only_for_target_stats() {
        assert_eq!(display_stat(7, false), "7");
        assert_eq!(display_stat(3, true), "3+");
    }

    #[test]
    fn next_stat_value_direction_matches_official_semantics() {
        assert_eq!(next_stat_value(StatKind::Ma, 6), 7);
        assert_eq!(next_stat_value(StatKind::St, 3), 4);
        assert_eq!(next_stat_value(StatKind::Av, 8), 9);
        assert_eq!(next_stat_value(StatKind::Ag, 3), 2);
        assert_eq!(next_stat_value(StatKind::Pa, 5), 4);
    }

    #[test]
    fn to_kpo_divides_raw_po_by_a_thousand() {
        assert_eq!(to_kpo(20_000), 20);
        assert_eq!(to_kpo(60_000), 60);
    }

    #[test]
    fn build_vm_wires_spp_remaining_level_and_urls() {
        let player = sample_player();
        let catalog = SkillCatalogAdapter::new(std::sync::Arc::new(InMemoryReferenceRepository::load_for_tests()));
        let routes = AppRoutes::default();

        let vm = build_vm(&player, &routes, "space1", &catalog).unwrap();

        assert_eq!(vm.spp_remaining, 30);
        assert_eq!(vm.stat_cards.len(), 5);
        assert!(vm.skill_picker_url.contains("roster_line_id=DEMO_GRANIT__PIETAILLE"));
        assert!(vm.skill_picker_url.contains("level=1"));
        assert!(vm.purchase_skill_url.contains("space1"));
        assert!(vm.purchase_skill_url.contains("p1"));
    }

    #[test]
    fn build_vm_reflects_level_advance_after_a_purchase() {
        let mut player = sample_player();
        player.acquired_skills.push(AcquiredSkill {
            skill_id: SkillId::try_new("block").unwrap(),
            skill_name: SkillName::try_new("Bloc").unwrap(),
            mode: AcquisitionMode::Chosen,
            spp_cost: SppCost::try_new(6).unwrap(),
            value_delta: ValueKpo(20_000),
        });
        let catalog = SkillCatalogAdapter::new(std::sync::Arc::new(InMemoryReferenceRepository::load_for_tests()));
        let routes = AppRoutes::default();

        let vm = build_vm(&player, &routes, "space1", &catalog).unwrap();

        // Niveau 2 désormais (une compétence déjà achetée) — le tarif doit suivre.
        assert!(vm.skill_picker_url.contains("level=2"));
        assert_eq!(vm.spp_remaining, 24);
    }

    #[test]
    fn build_vm_returns_none_for_unknown_position() {
        let mut player = sample_player();
        player.roster_line_id = RosterLineId::try_new("UNKNOWN".to_string()).unwrap();
        let catalog = SkillCatalogAdapter::new(std::sync::Arc::new(InMemoryReferenceRepository::load_for_tests()));
        let routes = AppRoutes::default();

        assert!(build_vm(&player, &routes, "space1", &catalog).is_none());
    }
}
