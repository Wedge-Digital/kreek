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
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::{MatchReportId, SpaceId};
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

    if !is_authorized(&state, &user, &space_id, &source).await {
        return StatusCode::FORBIDDEN.into_response();
    }

    build_recap_template(&space_id, &match_report_id, is_published, source, &state)
        .await
        .into_response()
}

/// Autorisé si l'utilisateur est admin d'espace, admin de la compétition du
/// rapport, ou coach de l'une des deux équipes concernées.
async fn is_authorized(state: &AppState, user: &User, space_id: &str, source: &RecapSource<'_>) -> bool {
    let user_id = user.id.to_string();

    let is_space_admin = match SpaceId::try_new(space_id) {
        Ok(sid) => matches!(
            state.spaces.space_repository.find_member_profile(&user.id, &sid).await,
            Ok(Some(SpaceProfile::SpaceAdmin))
        ),
        Err(_) => false,
    };
    if is_space_admin {
        return true;
    }

    let is_comp_admin = state
        .match_report
        .competition_data
        .is_competition_admin(&source.competition_id, &user_id)
        .await
        .unwrap_or(false);
    if is_comp_admin {
        return true;
    }

    let is_home_coach = state
        .match_report
        .team_data
        .is_coach_of_team(&source.home_team_id, &user_id)
        .await
        .unwrap_or(false);
    let is_away_coach = state
        .match_report
        .team_data
        .is_coach_of_team(&source.away_team_id, &user_id)
        .await
        .unwrap_or(false);

    is_home_coach || is_away_coach
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

    let cmd = PublishMatchReportCommand { match_report_id: mr_id, published_by: user.id };

    match publish_match_report_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        &state.match_report.event_bus,
    )
    .await
    {
        Ok(()) => {
            let url = AppRoutes::default().match_report.recap(&space_id, &match_report_id);
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
