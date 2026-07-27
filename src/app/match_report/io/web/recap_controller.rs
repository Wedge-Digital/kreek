use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_published::MatchReportPublished;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::MatchAction;
use crate::app::match_report::io::web::builders::{
    build_performance_rows, build_round_context_vm, build_submitted_by, build_team_banner,
    PerformanceRowVm, RoundContextVm, TeamBannerVm,
};
use crate::app::match_report::io::web::view_models::{
    GainsFanVm, HalfTimelineVm, InjuryRowVm, MatchResultVm, MvpRowVm,
};
use crate::app::match_report::use_cases::publish_match_report_use_case::{
    self, PublishMatchReportCommand, PublishMatchReportError,
};
use crate::app::match_report::ports::{ICompetitionDataPort, ISpaceAdminPort, ITeamDataPort};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::MatchReportId;
use crate::app::shared_kernel::user::User;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};

// ── Template ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "recap.html")]
pub struct RecapTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub is_published: bool,
    pub round_context: Option<RoundContextVm>,
    pub submitted_by: Option<String>,
    pub home_banner: TeamBannerVm,
    pub away_banner: TeamBannerVm,
    pub result: MatchResultVm,
    pub gains_fan: GainsFanVm,
    pub timeline_halves: Vec<HalfTimelineVm>,
    pub mvps: Vec<MvpRowVm>,
    pub injuries: Vec<InjuryRowVm>,
    pub performances: Vec<PerformanceRowVm>,
    pub publish_url: String,
    pub back_to_step5_url: String,
    pub competition_url: String,
    pub home_team_detail_url: String,
}

impl IntoResponse for RecapTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Vue neutre sur les champs communs à ReadyToPublish et Published — évite de dupliquer
/// la composition de VMs pour les deux états (Published = ReadyToPublish + published_*).
struct RecapSource<'a> {
    home_team_id: String,
    away_team_id: String,
    competition_id: String,
    season_id: String,
    round_id: String,
    created_by: String,
    home_actions: &'a [MatchAction],
    away_actions: &'a [MatchAction],
    home_gain_kpo: u32,
    away_gain_kpo: u32,
    home_fan_mod: i8,
    away_fan_mod: i8,
    summary_title: Option<String>,
    summary_body: Option<String>,
}

impl<'a> RecapSource<'a> {
    fn from_rtp(rtp: &'a MatchReportReadyToPublish) -> Self {
        Self {
            home_team_id: rtp.home_team_id.to_string(),
            away_team_id: rtp.away_team_id.to_string(),
            competition_id: rtp.competition_id.to_string(),
            season_id: rtp.season_id.to_string(),
            round_id: rtp.round_id.to_string(),
            created_by: rtp.created_by.to_string(),
            home_actions: &rtp.home_actions,
            away_actions: &rtp.away_actions,
            home_gain_kpo: rtp.home_gain.into_inner(),
            away_gain_kpo: rtp.away_gain.into_inner(),
            home_fan_mod: rtp.home_fan_mod.into_inner(),
            away_fan_mod: rtp.away_fan_mod.into_inner(),
            summary_title: rtp.summary_title.clone(),
            summary_body: rtp.summary_body.clone(),
        }
    }

    fn from_published(p: &'a MatchReportPublished) -> Self {
        Self {
            home_team_id: p.home_team_id.to_string(),
            away_team_id: p.away_team_id.to_string(),
            competition_id: p.competition_id.to_string(),
            season_id: p.season_id.to_string(),
            round_id: p.round_id.to_string(),
            created_by: p.created_by.to_string(),
            home_actions: &p.home_actions,
            away_actions: &p.away_actions,
            home_gain_kpo: p.home_gain.into_inner(),
            away_gain_kpo: p.away_gain.into_inner(),
            home_fan_mod: p.home_fan_mod.into_inner(),
            away_fan_mod: p.away_fan_mod.into_inner(),
            summary_title: p.summary_title.clone(),
            summary_body: p.summary_body.clone(),
        }
    }
}

// ── GET ───────────────────────────────────────────────────────────────────────

pub async fn get_recap(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let mr_state = match state.match_report.match_report_repo.find_by_id(&match_report_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_recap find_by_id {match_report_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let (source, is_published) = match &mr_state {
        MatchReportState::ReadyToPublish(rtp) => (RecapSource::from_rtp(rtp), false),
        MatchReportState::Published(p) => (RecapSource::from_published(p), true),
        MatchReportState::Draft(_) | MatchReportState::PreMatch(_) => {
            return StatusCode::NOT_FOUND.into_response();
        }
        MatchReportState::Cancelled(_) => return StatusCode::GONE.into_response(),
    };

    let scope = RecapScope::from_source(&source);
    if !is_authorized(&RecapAuthDeps::from_state(&state), &user, &space_id, &scope).await {
        return StatusCode::FORBIDDEN.into_response();
    }

    build_recap_template(&space_id, &match_report_id, is_published, source, &state)
        .await
        .into_response()
}

// ── Autorisation ──────────────────────────────────────────────────────────────

/// Dépendances du contrôle d'accès : les trois ports réellement consultés,
/// plutôt que `AppState` entier. C'est ce qui rend la règle testable — `AppState`
/// n'est pas constructible en test unitaire.
struct RecapAuthDeps<'a> {
    space_admin:      &'a dyn ISpaceAdminPort,
    competition_data: &'a dyn ICompetitionDataPort,
    team_data:        &'a dyn ITeamDataPort,
}

impl<'a> RecapAuthDeps<'a> {
    fn from_state(state: &'a AppState) -> Self {
        Self {
            space_admin:      state.match_report.space_admin.as_ref(),
            competition_data: state.match_report.competition_data.as_ref(),
            team_data:        state.match_report.team_data.as_ref(),
        }
    }
}

/// Ce sur quoi porte l'autorisation. Extrait de `RecapSource`, qui emprunte les
/// actions du match et serait pénible à fabriquer en test.
struct RecapScope {
    competition_id: String,
    home_team_id:   String,
    away_team_id:   String,
}

impl RecapScope {
    fn from_source(source: &RecapSource<'_>) -> Self {
        Self {
            competition_id: source.competition_id.clone(),
            home_team_id:   source.home_team_id.clone(),
            away_team_id:   source.away_team_id.clone(),
        }
    }

    /// Les états sans recap consultable sont traduits en statut HTTP, avec les
    /// mêmes codes que ceux produits par le use case de publication.
    fn from_report_state(state: &MatchReportState) -> Result<Self, StatusCode> {
        match state {
            MatchReportState::ReadyToPublish(rtp) => {
                Ok(Self::from_source(&RecapSource::from_rtp(rtp)))
            }
            MatchReportState::Published(p) => Ok(Self::from_source(&RecapSource::from_published(p))),
            MatchReportState::Draft(_) | MatchReportState::PreMatch(_) => {
                Err(StatusCode::NOT_FOUND)
            }
            MatchReportState::Cancelled(_) => Err(StatusCode::GONE),
        }
    }
}

/// Autorisé si l'utilisateur est admin d'espace, admin de la compétition du
/// rapport, ou coach de l'une des deux équipes concernées.
async fn is_authorized(
    deps:     &RecapAuthDeps<'_>,
    user:     &User,
    space_id: &str,
    scope:    &RecapScope,
) -> bool {
    let user_id = user.id.to_string();
    if deps.space_admin.is_space_admin(&user_id, space_id).await {
        return true;
    }
    if is_competition_admin(deps, &scope.competition_id, &user_id).await {
        return true;
    }
    is_coach_of_either_team(deps, scope, &user_id).await
}

async fn is_competition_admin(
    deps:           &RecapAuthDeps<'_>,
    competition_id: &str,
    user_id:        &str,
) -> bool {
    deps.competition_data
        .is_competition_admin(competition_id, user_id)
        .await
        .unwrap_or(false)
}

/// Une erreur de port vaut « pas coach » : un contrôle d'accès échoue fermé.
async fn is_coach_of_either_team(
    deps:    &RecapAuthDeps<'_>,
    scope:   &RecapScope,
    user_id: &str,
) -> bool {
    let (home, away) = tokio::join!(
        deps.team_data.is_coach_of_team(&scope.home_team_id, user_id),
        deps.team_data.is_coach_of_team(&scope.away_team_id, user_id),
    );
    home.unwrap_or(false) || away.unwrap_or(false)
}

async fn build_recap_template(
    space_id: &str,
    match_report_id: &str,
    is_published: bool,
    source: RecapSource<'_>,
    state: &AppState,
) -> RecapTemplate {
    let (home_info, away_info) = tokio::join!(
        state.match_report.team_data.find_team_info(&source.home_team_id),
        state.match_report.team_data.find_team_info(&source.away_team_id),
    );
    let home_info = home_info.unwrap_or_default();
    let away_info = away_info.unwrap_or_default();
    let home_team_name = home_info.team_name.clone();
    let away_team_name = away_info.team_name.clone();

    let result = MatchResultVm::from_domain(
        source.home_actions,
        source.away_actions,
        source.summary_title.clone(),
        source.summary_body.clone(),
    );

    let (round_context, performances, submitted_by) = tokio::join!(
        build_round_context_vm(state.match_report.competition_data.as_ref(), &source.season_id, &source.round_id),
        build_performance_rows(
            state.match_report.spp_calculator.as_ref(),
            source.home_actions,
            source.away_actions,
            &home_info.roster_id,
            &away_info.roster_id,
        ),
        build_submitted_by(state.match_report.coach_data.as_ref(), &source.created_by),
    );

    let routes = AppRoutes::default();
    RecapTemplate {
        app_routes: routes,
        space_id: space_id.to_string(),
        match_report_id: match_report_id.to_string(),
        is_published,
        round_context,
        submitted_by,
        home_banner: build_team_banner(home_info, result.home_score, result.away_score),
        away_banner: build_team_banner(away_info, result.away_score, result.home_score),
        gains_fan: GainsFanVm {
            home_gain_kpo: source.home_gain_kpo,
            away_gain_kpo: source.away_gain_kpo,
            home_fan_mod: source.home_fan_mod,
            away_fan_mod: source.away_fan_mod,
        },
        timeline_halves: HalfTimelineVm::all_from_domain(source.home_actions, source.away_actions),
        mvps: MvpRowVm::all_from_domain(
            source.home_actions,
            source.away_actions,
            &home_team_name,
            &away_team_name,
        ),
        injuries: InjuryRowVm::all_from_domain(source.home_actions, source.away_actions),
        performances,
        publish_url: routes.match_report.recap_publish(space_id, match_report_id),
        back_to_step5_url: routes.match_report.step5(space_id, match_report_id),
        competition_url: routes.competitions.competition_detail(space_id, &source.competition_id, &source.season_id),
        home_team_detail_url: routes.teams.team_detail(space_id, &source.home_team_id),
        result,
    }
}

// ── POST ──────────────────────────────────────────────────────────────────────

pub async fn post_publish(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(mr_id) = MatchReportId::try_new(&match_report_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    if let Err(status) = authorize_publish(&state, &user, &space_id, &match_report_id).await {
        return status.into_response();
    }
    let cmd = PublishMatchReportCommand { match_report_id: mr_id, published_by: user.id };
    publish_response(execute_publish(&state, cmd).await, &space_id, &match_report_id)
}

/// Charge le rapport et vérifie que l'utilisateur a le droit de le publier.
///
/// Mêmes droits que la consultation du recap : sans ce contrôle, tout
/// utilisateur connecté connaissant un `match_report_id` pouvait publier le
/// rapport d'autrui.
async fn authorize_publish(
    state:           &AppState,
    user:            &User,
    space_id:        &str,
    match_report_id: &str,
) -> Result<(), StatusCode> {
    let scope = load_publish_scope(state, match_report_id).await?;
    match is_authorized(&RecapAuthDeps::from_state(state), user, space_id, &scope).await {
        true => Ok(()),
        false => Err(StatusCode::FORBIDDEN),
    }
}

async fn load_publish_scope(
    state:           &AppState,
    match_report_id: &str,
) -> Result<RecapScope, StatusCode> {
    let mr_state = state
        .match_report
        .match_report_repo
        .find_by_id(match_report_id)
        .await
        .map_err(|e| {
            tracing::error!("authorize_publish find_by_id {match_report_id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;
    RecapScope::from_report_state(&mr_state)
}

async fn execute_publish(
    state: &AppState,
    cmd:   PublishMatchReportCommand,
) -> Result<(), PublishMatchReportError> {
    publish_match_report_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        &state.match_report.event_bus,
    )
    .await
}

fn publish_response(
    result:          Result<(), PublishMatchReportError>,
    space_id:        &str,
    match_report_id: &str,
) -> Response {
    match result {
        Ok(()) => {
            let url = AppRoutes::default().match_report.recap(space_id, match_report_id);
            Redirect::to(&url).into_response()
        }
        Err(PublishMatchReportError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(PublishMatchReportError::AlreadyPublished) => StatusCode::CONFLICT.into_response(),
        Err(PublishMatchReportError::Cancelled) => StatusCode::GONE.into_response(),
        Err(PublishMatchReportError::Repository(e)) => {
            tracing::error!("post_publish: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod authorization_tests {
    use super::*;
    use crate::app::match_report::ports::{
        JourneymanPositionDto, RosterPositionDto, RoundContextDto, TeamInfoDto, TierRulesDto,
    };
    use crate::app::shared_kernel::coach_name::CoachName;
    use crate::app::shared_kernel::common_types::UserId;
    use crate::app::shared_kernel::email::Email;

    const HOME: &str = "home-team";
    const AWAY: &str = "away-team";
    const SPACE: &str = "space-1";

    struct FakeSpaceAdmin(bool);
    #[async_trait::async_trait]
    impl ISpaceAdminPort for FakeSpaceAdmin {
        async fn is_space_admin(&self, _: &str, _: &str) -> bool {
            self.0
        }
    }

    struct FakeCompetitionData(Result<bool, String>);
    #[async_trait::async_trait]
    impl ICompetitionDataPort for FakeCompetitionData {
        async fn is_competition_admin(&self, _: &str, _: &str) -> Result<bool, String> {
            self.0.clone()
        }
        async fn find_tier_rules_for_roster(&self, _: &str, _: &str) -> Option<TierRulesDto> {
            None
        }
        async fn find_round_context(&self, _: &str, _: &str) -> Option<RoundContextDto> {
            None
        }
    }

    /// `coached` liste les équipes dont l'utilisateur est coach. `fails` simule
    /// une indisponibilité du port, pour vérifier qu'on échoue fermé.
    struct FakeTeamData {
        coached: Vec<&'static str>,
        fails:   bool,
    }
    #[async_trait::async_trait]
    impl ITeamDataPort for FakeTeamData {
        async fn is_coach_of_team(&self, team_id: &str, _: &str) -> Result<bool, String> {
            if self.fails {
                return Err("port indisponible".to_string());
            }
            Ok(self.coached.contains(&team_id))
        }
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn find_team_info(&self, _: &str) -> Option<TeamInfoDto> {
            None
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_journeyman_position(&self, _: &str) -> Option<JourneymanPositionDto> {
            None
        }
        async fn find_roster_positions(&self, _: &str) -> Vec<RosterPositionDto> {
            vec![]
        }
    }

    fn user() -> User {
        User::new(
            UserId::new(),
            CoachName::try_new("Testeur".to_string()).unwrap(),
            None,
            Email::try_new("testeur@example.com".to_string()).unwrap(),
            String::new(),
        )
    }

    fn scope() -> RecapScope {
        RecapScope {
            competition_id: "comp-1".to_string(),
            home_team_id:   HOME.to_string(),
            away_team_id:   AWAY.to_string(),
        }
    }

    async fn authorize(
        space_admin: bool,
        comp_admin:  Result<bool, String>,
        team_data:   FakeTeamData,
    ) -> bool {
        let space_admin = FakeSpaceAdmin(space_admin);
        let competition_data = FakeCompetitionData(comp_admin);
        let deps = RecapAuthDeps {
            space_admin:      &space_admin,
            competition_data: &competition_data,
            team_data:        &team_data,
        };
        is_authorized(&deps, &user(), SPACE, &scope()).await
    }

    fn coaching(teams: Vec<&'static str>) -> FakeTeamData {
        FakeTeamData { coached: teams, fails: false }
    }

    #[tokio::test]
    async fn admin_d_espace_est_autorise() {
        assert!(authorize(true, Ok(false), coaching(vec![])).await);
    }

    #[tokio::test]
    async fn admin_de_competition_est_autorise() {
        assert!(authorize(false, Ok(true), coaching(vec![])).await);
    }

    #[tokio::test]
    async fn coach_de_l_equipe_domicile_est_autorise() {
        assert!(authorize(false, Ok(false), coaching(vec![HOME])).await);
    }

    #[tokio::test]
    async fn coach_de_l_equipe_exterieure_est_autorise() {
        assert!(authorize(false, Ok(false), coaching(vec![AWAY])).await);
    }

    /// Le trou de sécurité que cette carte ferme : un utilisateur connecté mais
    /// étranger aux deux équipes ne doit pas pouvoir publier.
    #[tokio::test]
    async fn utilisateur_etranger_aux_deux_equipes_est_refuse() {
        assert!(!authorize(false, Ok(false), coaching(vec!["autre-equipe"])).await);
    }

    #[tokio::test]
    async fn erreur_du_port_competition_ne_donne_pas_l_acces() {
        assert!(!authorize(false, Err("indisponible".into()), coaching(vec![])).await);
    }

    #[tokio::test]
    async fn erreur_du_port_equipe_ne_donne_pas_l_acces() {
        let failing = FakeTeamData { coached: vec![HOME], fails: true };
        assert!(!authorize(false, Ok(false), failing).await);
    }
}
