use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::player::{Player, PlayerId};
use crate::app::players::io::web::purchase_skill_controller::can_spend_spp;
use crate::app::players::ports::TeamRosterInfoDto;
use crate::app::players::use_cases::match_history_service::{
    build_match_history, MatchHistoryAction, MatchHistoryActionKind, MatchHistoryEntry,
};
use crate::app::players::use_cases::player_stats_service::{self, ResolvedPlayerStats};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────

pub struct MatchActionLineVm {
    pub icon: &'static str,
    pub label: &'static str,
    pub spp: Option<u32>,
}

pub struct MatchHistoryCardVm {
    pub opponent_name: String,
    pub round_label: String,
    pub result_label: &'static str,
    pub result_css: &'static str,
    pub team_score: u8,
    pub opponent_score: u8,
    pub actions: Vec<MatchActionLineVm>,
    pub subtotal_spp: u32,
}

pub struct PlayerDetailVm {
    pub player_id: String,
    pub team_id: String,
    pub team_name: String,
    pub name: String,
    pub jersey: Option<i16>,
    pub position_name: String,
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
    pub base_skills: Vec<String>,
    pub acquired_skills: Vec<String>,
    pub value_formatted: String,
    pub spp_earned: u32,
    pub spp_spent: u32,
    pub spp_reserve: u32,
    pub spp_percent: u8,
    pub matches_played: u16,
    pub career_touchdowns: u16,
    pub career_passes: u16,
    pub career_interceptions: u16,
    pub career_casualties: u16,
    pub career_mvps: u16,
    pub can_customise: bool,
    pub match_history: Vec<MatchHistoryCardVm>,
    pub right_panel_widget_url: String,
    pub can_spend: bool,
}

fn action_line_vm(action: &MatchHistoryAction) -> MatchActionLineVm {
    let (icon, label) = match action.kind {
        MatchHistoryActionKind::Touchdown => ("🏈", "Touchdown"),
        MatchHistoryActionKind::Pass => ("🎯", "Passe réussie"),
        MatchHistoryActionKind::Interception => ("🛡️", "Interception"),
        MatchHistoryActionKind::Casualty => ("🩸", "Sortie infligée"),
        MatchHistoryActionKind::Mvp => ("⭐", "MVP"),
        MatchHistoryActionKind::Foul => ("🟨", "Faute"),
        MatchHistoryActionKind::Injury => ("🤕", "Blessure"),
    };
    MatchActionLineVm {
        icon,
        label,
        spp: action.spp_earned,
    }
}

fn match_history_card_vm(entry: MatchHistoryEntry) -> MatchHistoryCardVm {
    let (result_label, result_css) = match entry.team_score.cmp(&entry.opponent_score) {
        std::cmp::Ordering::Greater => ("Victoire", "green"),
        std::cmp::Ordering::Less => ("Défaite", "red"),
        std::cmp::Ordering::Equal => ("Nul", "dark"),
    };
    let subtotal_spp = entry.actions.iter().filter_map(|a| a.spp_earned).sum();
    MatchHistoryCardVm {
        opponent_name: entry.opponent_team_name,
        round_label: entry.round_label,
        result_label,
        result_css,
        team_score: entry.team_score,
        opponent_score: entry.opponent_score,
        actions: entry.actions.iter().map(action_line_vm).collect(),
        subtotal_spp,
    }
}

async fn check_admin_rights(
    state: &AppState,
    coach_id: &CoachId,
    coach_name: &str,
    space_id: &SpaceId,
    team: &TeamRosterInfoDto,
) -> bool {
    let is_space_admin = matches!(
        state
            .players
            .space_member_port
            .find_member_profile(coach_id, space_id)
            .await,
        Some(SpaceProfile::SpaceAdmin)
    );
    if is_space_admin {
        return true;
    }
    let Some(competition_id) = &team.competition_id else {
        return false;
    };
    let coach_id_str = coach_id.to_string();
    match state
        .players
        .competition_port
        .find_admin_info(competition_id)
        .await
    {
        Some(info) => {
            info.admin_ids.contains(&coach_id_str)
                || info.admin_names.contains(&coach_name.to_string())
        }
        None => false,
    }
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "player-detail.html")]
pub struct PlayerDetailTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub vm: PlayerDetailVm,
}

impl IntoResponse for PlayerDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("player_detail template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn player_detail_controller(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let player = match load_player(&state, &player_id).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let events = match load_events(&state, &player).await {
        Ok(e) => e,
        Err(resp) => return resp,
    };
    let team = match load_team(&state, &player).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let space_id_vo = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let coach_name = user.coach_name.clone().into_inner();
    let can_customise =
        check_admin_rights(&state, &user.id, &coach_name, &space_id_vo, &team).await;

    let can_spend =
        team.in_player_improvement_phase && can_spend_spp(&state, &user, &space_id_vo, &team).await;
    let app_routes = AppRoutes::default();
    // Le panneau droit charge toujours le journal par défaut — c'est le bouton
    // "Activer la dépense de SPP" (rendu par le journal lui-même si can_spend,
    // à la place du bandeau verrouillé) qui bascule vers le mode dépense.
    let right_panel_widget_url = format!(
        "{}?can_spend={}",
        app_routes
            .players
            .evolution_journal_widget(&space_id, &player_id),
        can_spend,
    );

    let vm = build_vm(
        &state,
        &player,
        &events,
        &team,
        can_customise,
        right_panel_widget_url,
        can_spend,
    );

    PlayerDetailTemplate {
        app_routes,
        space_id,
        vm,
    }
    .into_response()
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
            tracing::error!("player_detail_controller find_by_id: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

async fn load_events(
    state: &AppState,
    player: &Player,
) -> Result<Vec<crate::app::players::domain::events::PlayerDomainEvent>, Response> {
    state
        .players
        .repository
        .find_events_by_id(&player.id)
        .await
        .map_err(|e| {
            tracing::error!("player_detail_controller find_events_by_id: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}

async fn load_team(state: &AppState, player: &Player) -> Result<TeamRosterInfoDto, Response> {
    match state
        .players
        .roster_port
        .find_team_info(&player.team_id.0)
        .await
    {
        Some(t) => Ok(t),
        None => Err(StatusCode::NOT_FOUND.into_response()),
    }
}

struct SppBreakdown {
    earned: u32,
    spent: u32,
    reserve: u32,
    percent: u8,
}

fn compute_spp_breakdown(player: &Player) -> SppBreakdown {
    let earned = player.spp.0;
    let reserve = player.spp_remaining();
    let spent = earned.saturating_sub(reserve);
    let percent = if earned == 0 {
        0
    } else {
        ((spent * 100) / earned).min(100) as u8
    };
    SppBreakdown {
        earned,
        spent,
        reserve,
        percent,
    }
}

fn build_vm(
    state: &AppState,
    player: &Player,
    events: &[crate::app::players::domain::events::PlayerDomainEvent],
    team: &TeamRosterInfoDto,
    can_customise: bool,
    right_panel_widget_url: String,
    can_spend: bool,
) -> PlayerDetailVm {
    let catalog = state.players.skill_catalog.as_ref();
    let stats =
        player_stats_service::resolve_stats(player, catalog).unwrap_or(ResolvedPlayerStats {
            ma: 0,
            st: 0,
            ag: 0,
            pa: 0,
            av: 0,
        });
    let base_skills = player
        .base_skills
        .iter()
        .map(|id| {
            catalog
                .find_skill(id.as_ref())
                .map(|s| s.name)
                .unwrap_or_else(|| id.as_ref().to_string())
        })
        .collect();
    let acquired_skills = player
        .acquired_skills
        .iter()
        .map(|s| s.skill_name.to_string())
        .collect();
    let spp = compute_spp_breakdown(player);
    let match_history = build_match_history(events)
        .into_iter()
        .map(match_history_card_vm)
        .collect();

    PlayerDetailVm {
        player_id: player.id.0.clone(),
        team_id: player.team_id.0.clone(),
        team_name: team.team_name.clone(),
        name: player.position_name.to_string(),
        jersey: player.jersey.map(|j| j.into_inner() as i16),
        position_name: player.position_name.to_string(),
        ma: stats.ma,
        st: stats.st,
        ag: stats.ag,
        pa: stats.pa,
        av: stats.av,
        base_skills,
        acquired_skills,
        value_formatted: format!("{} kPo", player.value.0),
        spp_earned: spp.earned,
        spp_spent: spp.spent,
        spp_reserve: spp.reserve,
        spp_percent: spp.percent,
        matches_played: player.matches_played.0,
        career_touchdowns: player.career_touchdowns.0,
        career_passes: player.career_passes.0,
        career_interceptions: player.career_interceptions.0,
        career_casualties: player.career_casualties.0,
        career_mvps: player.career_mvps.0,
        can_customise,
        match_history,
        right_panel_widget_url,
        can_spend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::player::{
        AcquiredSkill, AcquisitionMode, Spp, TeamId, ValueKpo,
    };
    use crate::app::players::domain::value_objects::{
        PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
    };
    use crate::app::shared_kernel::identity::ids::SpaceId;

    fn sample_player_with_spp(spp_earned: u32, skill_costs: &[u8]) -> Player {
        let created = crate::app::players::domain::events::PlayerDomainEvent::PlayerCreated {
            player_id: PlayerId("p1".into()),
            team_id: TeamId("t1".into()),
            space_id: SpaceId::new(),
            position_name: PositionNameVo::try_new("Frappeur".to_string()).unwrap(),
            roster_line_id: RosterLineId::try_new("BLITZER".to_string()).unwrap(),
            jersey: None,
            base_skills: vec![],
            starting_spp: Spp(0),
            starting_value: ValueKpo(100),
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
    fn compute_spp_breakdown_derives_reserve_and_percent() {
        let player = sample_player_with_spp(31, &[6, 12]);
        let spp = compute_spp_breakdown(&player);
        assert_eq!(spp.earned, 31);
        assert_eq!(spp.spent, 18);
        assert_eq!(spp.reserve, 13);
        assert_eq!(spp.percent, 58);
    }

    #[test]
    fn compute_spp_breakdown_zero_earned_gives_zero_percent() {
        let player = sample_player_with_spp(0, &[]);
        let spp = compute_spp_breakdown(&player);
        assert_eq!(spp.percent, 0);
    }

    #[test]
    fn action_line_vm_maps_touchdown_icon_and_label() {
        let action = MatchHistoryAction {
            kind: MatchHistoryActionKind::Touchdown,
            spp_earned: Some(3),
        };
        let vm = action_line_vm(&action);
        assert_eq!(vm.icon, "🏈");
        assert_eq!(vm.label, "Touchdown");
        assert_eq!(vm.spp, Some(3));
    }

    #[test]
    fn action_line_vm_foul_has_no_spp() {
        let action = MatchHistoryAction {
            kind: MatchHistoryActionKind::Foul,
            spp_earned: None,
        };
        let vm = action_line_vm(&action);
        assert_eq!(vm.icon, "🟨");
        assert_eq!(vm.spp, None);
    }

    #[test]
    fn match_history_card_vm_derives_victory_result() {
        let entry = MatchHistoryEntry {
            match_report_id: "mr1".into(),
            round_label: "Journée 1".into(),
            opponent_team_name: "Bone Crushers".into(),
            team_score: 3,
            opponent_score: 1,
            actions: vec![
                crate::app::players::use_cases::match_history_service::MatchHistoryAction {
                    kind: MatchHistoryActionKind::Touchdown,
                    spp_earned: Some(3),
                },
                crate::app::players::use_cases::match_history_service::MatchHistoryAction {
                    kind: MatchHistoryActionKind::Foul,
                    spp_earned: None,
                },
            ],
        };
        let vm = match_history_card_vm(entry);
        assert_eq!(vm.result_label, "Victoire");
        assert_eq!(vm.result_css, "green");
        assert_eq!(vm.subtotal_spp, 3);
    }

    #[test]
    fn match_history_card_vm_derives_defeat_and_draw() {
        let defeat = MatchHistoryEntry {
            match_report_id: "mr1".into(),
            round_label: "J1".into(),
            opponent_team_name: "X".into(),
            team_score: 0,
            opponent_score: 2,
            actions: vec![],
        };
        assert_eq!(match_history_card_vm(defeat).result_label, "Défaite");

        let draw = MatchHistoryEntry {
            match_report_id: "mr2".into(),
            round_label: "J2".into(),
            opponent_team_name: "X".into(),
            team_score: 1,
            opponent_score: 1,
            actions: vec![],
        };
        assert_eq!(match_history_card_vm(draw).result_label, "Nul");
    }
}
