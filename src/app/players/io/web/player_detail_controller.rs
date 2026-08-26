use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::customisation_basket::is_expired;
use crate::app::players::domain::player::{Player, PlayerId};
use crate::app::players::io::web::player_loader::charger_joueur;
use crate::app::players::io::web::purchase_skill_controller::can_spend_spp;
use crate::app::players::ports::{
    CustomisationBasketState, ICustomisationBasketRepository, TeamRosterInfoDto,
};
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

/// Qui a le droit de customiser un joueur : **admin d'espace ou admin de la
/// compétition**, et personne d'autre. Le coach de l'équipe en est exclu — un
/// coach qui s'ajouterait des compétences gratuitement ne serait pas la même
/// fonctionnalité.
///
/// C'est `can_spend_spp` qui, lui, est explicitement « étendu au coach ». Les
/// deux fonctions se ressemblent assez pour qu'on les confonde ; celle-ci
/// portait auparavant le nom générique `check_admin_rights`, qui ne disait pas
/// ce qu'il autorisait.
pub async fn can_customise(
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

    let player = match charger_joueur(&state, &player_id).await {
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
    let customisable = can_customise(&state, &user.id, &coach_name, &space_id_vo, &team).await;

    let can_spend =
        team.in_player_improvement_phase && can_spend_spp(&state, &user, &space_id_vo, &team).await;
    let app_routes = AppRoutes::default();
    let right_panel_widget_url =
        right_panel_url(&state, &space_id, &player_id, customisable, can_spend).await;

    let vm = build_vm(
        &state,
        &player,
        &events,
        &team,
        customisable,
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

// ── Choix de l'occupant du slot droit ─────────────────────────────────────────

/// Ce que la fiche installe dans `#pd-right-panel` à son ouverture.
#[derive(Debug, PartialEq)]
pub enum RightPanel {
    /// Le journal des évolutions — cas par défaut. `abandoned` signale une
    /// saisie périmée supprimée au passage, que le journal annonce discrètement.
    Journal { abandoned: bool },
    /// Le mode customisation, parce qu'un panier est ouvert sur ce joueur.
    Customisation,
}

/// **Panier existant *et* droit de customiser**, jamais l'un sans l'autre.
///
/// Le panier est propre au *joueur*, pas à son auteur : sans la seconde
/// condition, un panier laissé ouvert par un commissaire ferait apparaître le
/// mode administration au coach qui ouvrirait la même fiche.
///
/// La péremption se juge ici et pas dans un `WHERE` : c'est la seule lecture du
/// panier sur le chemin d'ouverture, et un panier de la veille qui rouvrirait
/// le mode serait pire qu'un panier perdu.
pub fn choose_right_panel(
    can_customise: bool,
    basket: Option<&CustomisationBasketState>,
    now: time::OffsetDateTime,
) -> RightPanel {
    if !can_customise {
        return RightPanel::Journal { abandoned: false };
    }
    match basket {
        None => RightPanel::Journal { abandoned: false },
        Some(etat) if is_expired(etat.updated_at, now) => RightPanel::Journal { abandoned: true },
        Some(_) => RightPanel::Customisation,
    }
}

/// Traduit la décision en URL de widget, en supprimant au passage le panier
/// périmé. Un `GET` qui écrit, assumé : la suppression est idempotente, et la
/// seule alternative serait une tâche de ménage pour un état qui ne gêne que
/// l'utilisateur qui le rencontre.
async fn right_panel_url(
    state: &AppState,
    space_id: &str,
    player_id: &str,
    can_customise: bool,
    can_spend: bool,
) -> String {
    let repo = state.players.customisation_basket_repository.as_ref();
    // Inutile d'interroger la base pour un utilisateur qui n'y a pas droit.
    let panier = match can_customise {
        true => repo.load(player_id).await.unwrap_or(None),
        false => None,
    };
    let decision = choose_right_panel(
        can_customise,
        panier.as_ref(),
        time::OffsetDateTime::now_utc(),
    );
    if let RightPanel::Journal { abandoned: true } = decision {
        purger_panier(repo, player_id).await;
    }
    url_du_panneau(decision, space_id, player_id, can_spend)
}

fn url_du_panneau(
    decision: RightPanel,
    space_id: &str,
    player_id: &str,
    can_spend: bool,
) -> String {
    let routes = AppRoutes::default();
    match decision {
        RightPanel::Customisation => routes.players.customisation_widget(space_id, player_id),
        RightPanel::Journal { abandoned } => format!(
            "{}?can_spend={can_spend}&abandoned={abandoned}",
            routes.players.evolution_journal_widget(space_id, player_id),
        ),
    }
}

/// L'échec de purge n'interrompt rien : la fiche s'ouvre, le panier périmé sera
/// revu au prochain passage. Refuser d'afficher la page pour ça serait pire que
/// le mal.
async fn purger_panier(repo: &dyn ICustomisationBasketRepository, player_id: &str) {
    if let Err(e) = repo.delete(player_id).await {
        tracing::error!("player_detail_controller purge panier {player_id}: {e:?}");
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
                from_match: None,
            });
        }
        player
    }

    // ── Choix de l'occupant du slot droit ─────────────────────────────────────

    fn maintenant() -> time::OffsetDateTime {
        time::OffsetDateTime::UNIX_EPOCH + time::Duration::days(100)
    }

    fn panier(age_heures: i64) -> CustomisationBasketState {
        CustomisationBasketState {
            player_id: "p1".into(),
            space_id: "s1".into(),
            state: serde_json::json!([]),
            version: 1,
            updated_at: maintenant() - time::Duration::hours(age_heures),
        }
    }

    /// Un panier ouvert rouvre le mode pour qui a le droit — c'est ce qui rend
    /// la customisation reprenable après un rechargement complet.
    #[test]
    fn un_panier_ouvert_rouvre_le_mode_pour_un_commissaire() {
        assert_eq!(
            choose_right_panel(true, Some(&panier(1)), maintenant()),
            RightPanel::Customisation
        );
    }

    /// Le même panier, vu par un coach : fiche classique. Le panier est propre
    /// au joueur, pas à son auteur — sans cette condition le mode
    /// administration apparaîtrait à qui n'a pas le droit de le voir.
    #[test]
    fn le_meme_panier_laisse_la_fiche_classique_a_un_coach() {
        assert_eq!(
            choose_right_panel(false, Some(&panier(1)), maintenant()),
            RightPanel::Journal { abandoned: false }
        );
    }

    /// Un panier de plus de 24 h ne rouvre rien : il est abandonné, et dit.
    #[test]
    fn un_panier_perime_est_abandonne_avec_son_message() {
        assert_eq!(
            choose_right_panel(true, Some(&panier(25)), maintenant()),
            RightPanel::Journal { abandoned: true }
        );
        assert_eq!(
            choose_right_panel(true, Some(&panier(23)), maintenant()),
            RightPanel::Customisation
        );
    }

    /// Pas de panier, pas de message d'abandon : il n'y avait rien à perdre.
    #[test]
    fn sans_panier_le_journal_ne_signale_rien() {
        assert_eq!(
            choose_right_panel(true, None, maintenant()),
            RightPanel::Journal { abandoned: false }
        );
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
