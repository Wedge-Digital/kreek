use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::use_cases::competition_admin_access_service::{
    peut_administrer, AdminsDeLaCompetition,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

use crate::app::shared_kernel::identity::cloudinary;
use crate::common::initials::initials;
// ── Mock data structs ─────────────────────────────────────────────────────────

pub struct TeamCard {
    pub name: String,
    pub logo: Option<String>,
    pub roster: String,
    pub coach: String,
    pub tv: u32,
}

pub struct StatRow {
    pub rank: u32,
    pub coach: String,
    pub team: String,
    pub value: u32,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn mock_teams() -> Vec<TeamCard> {
    vec![
        TeamCard {
            name: "Les Guerriers du Nord".into(),
            logo: None,
            roster: "Nordiques".into(),
            coach: "CoachAlpha".into(),
            tv: 1850,
        },
        TeamCard {
            name: "Waaagh! FC".into(),
            logo: None,
            roster: "Orques".into(),
            coach: "WaaghMaster".into(),
            tv: 1720,
        },
        TeamCard {
            name: "Chaos United".into(),
            logo: None,
            roster: "Chaos".into(),
            coach: "ChaosBoss".into(),
            tv: 1680,
        },
        TeamCard {
            name: "Elfes Sylvains SC".into(),
            logo: None,
            roster: "Elfes Sylvains".into(),
            coach: "ElveRunner".into(),
            tv: 1540,
        },
        TeamCard {
            name: "Nains de Fer".into(),
            logo: None,
            roster: "Nains".into(),
            coach: "IronBeard".into(),
            tv: 1490,
        },
        TeamCard {
            name: "Skavens du Sous-sol".into(),
            logo: None,
            roster: "Skavens".into(),
            coach: "SkavRunner".into(),
            tv: 1310,
        },
        TeamCard {
            name: "Humains Ordinaires".into(),
            logo: None,
            roster: "Humains".into(),
            coach: "HumanCoach".into(),
            tv: 1200,
        },
        TeamCard {
            name: "Nécromants XI".into(),
            logo: None,
            roster: "Nécromants".into(),
            coach: "NecroMind".into(),
            tv: 1050,
        },
        TeamCard {
            name: "Orques Sauvages".into(),
            logo: None,
            roster: "Orques Sauvages".into(),
            coach: "GreenFist".into(),
            tv: 820,
        },
        TeamCard {
            name: "Halflings United".into(),
            logo: None,
            roster: "Halflings".into(),
            coach: "TinyCoach".into(),
            tv: 530,
        },
        TeamCard {
            name: "Les Intrépides Chevaliers de la Montagne Éternelle".into(),
            logo: None,
            roster: "Bretonniens".into(),
            coach: "LongNameCoach".into(),
            tv: 1150,
        },
    ]
}

fn mock_top_tds() -> Vec<StatRow> {
    vec![
        StatRow {
            rank: 1,
            coach: "CoachAlpha".into(),
            team: "Les Guerriers du Nord".into(),
            value: 18,
        },
        StatRow {
            rank: 2,
            coach: "WaaghMaster".into(),
            team: "Waaagh! FC".into(),
            value: 15,
        },
        StatRow {
            rank: 3,
            coach: "ChaosBoss".into(),
            team: "Chaos United".into(),
            value: 12,
        },
        StatRow {
            rank: 4,
            coach: "ElveRunner".into(),
            team: "Elfes Sylvains SC".into(),
            value: 11,
        },
        StatRow {
            rank: 5,
            coach: "IronBeard".into(),
            team: "Nains de Fer".into(),
            value: 9,
        },
    ]
}

fn mock_top_casualties() -> Vec<StatRow> {
    vec![
        StatRow {
            rank: 1,
            coach: "ChaosBoss".into(),
            team: "Chaos United".into(),
            value: 22,
        },
        StatRow {
            rank: 2,
            coach: "IronBeard".into(),
            team: "Nains de Fer".into(),
            value: 19,
        },
        StatRow {
            rank: 3,
            coach: "WaaghMaster".into(),
            team: "Waaagh! FC".into(),
            value: 17,
        },
        StatRow {
            rank: 4,
            coach: "CoachAlpha".into(),
            team: "Les Guerriers du Nord".into(),
            value: 14,
        },
        StatRow {
            rank: 5,
            coach: "SkavRunner".into(),
            team: "Skavens du Sous-sol".into(),
            value: 11,
        },
    ]
}

fn mock_flop_tds() -> Vec<StatRow> {
    vec![
        StatRow {
            rank: 1,
            coach: "TinyCoach".into(),
            team: "Halflings United".into(),
            value: 14,
        },
        StatRow {
            rank: 2,
            coach: "GreenFist".into(),
            team: "Orques Sauvages".into(),
            value: 13,
        },
        StatRow {
            rank: 3,
            coach: "NecroMind".into(),
            team: "Nécromants XI".into(),
            value: 11,
        },
        StatRow {
            rank: 4,
            coach: "HumanCoach".into(),
            team: "Humains Ordinaires".into(),
            value: 9,
        },
        StatRow {
            rank: 5,
            coach: "SkavRunner".into(),
            team: "Skavens du Sous-sol".into(),
            value: 8,
        },
    ]
}

fn mock_flop_casualties() -> Vec<StatRow> {
    vec![
        StatRow {
            rank: 1,
            coach: "TinyCoach".into(),
            team: "Halflings United".into(),
            value: 28,
        },
        StatRow {
            rank: 2,
            coach: "ElveRunner".into(),
            team: "Elfes Sylvains SC".into(),
            value: 21,
        },
        StatRow {
            rank: 3,
            coach: "NecroMind".into(),
            team: "Nécromants XI".into(),
            value: 18,
        },
        StatRow {
            rank: 4,
            coach: "GreenFist".into(),
            team: "Orques Sauvages".into(),
            value: 16,
        },
        StatRow {
            rank: 5,
            coach: "HumanCoach".into(),
            team: "Humains Ordinaires".into(),
            value: 12,
        },
    ]
}

// ── Full-page template ────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "competition-detail.html")]
pub struct CompetitionDetailTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub competition_name: String,
    pub competition_logo: Option<String>,
    pub competition_initials: String,
    pub season_name: String,
    pub admin_names: Vec<String>,
    pub is_admin: bool,
    pub hors_calendrier_interdit: bool,
    pub active_tab: &'static str,
    // tab content (only one is populated per request)
    pub top_tds: Vec<StatRow>,
    pub top_casualties: Vec<StatRow>,
    pub flop_tds: Vec<StatRow>,
    pub flop_casualties: Vec<StatRow>,
}

impl IntoResponse for CompetitionDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Fragment templates ────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "competition-tab-standings.html")]
pub struct StandingsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
}

#[derive(Template)]
#[template(path = "competition-tab-detailed-standings.html")]
pub struct DetailedStandingsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
}

#[derive(Template)]
#[template(path = "competition-tab-teams.html")]
pub struct TeamsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub season_id: String,
}

#[derive(Template)]
#[template(path = "competition-tab-stats.html")]
pub struct StatsTabTemplate {
    pub top_tds: Vec<StatRow>,
    pub top_casualties: Vec<StatRow>,
    pub flop_tds: Vec<StatRow>,
    pub flop_casualties: Vec<StatRow>,
}

impl IntoResponse for StandingsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
impl IntoResponse for DetailedStandingsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
impl IntoResponse for TeamsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
impl IntoResponse for StatsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Shared loader ─────────────────────────────────────────────────────────────

pub(crate) struct PageBase {
    pub(crate) competition_name: String,
    pub(crate) competition_logo: Option<String>,
    pub(crate) competition_initials: String,
    pub(crate) season_name: String,
    pub(crate) admin_names: Vec<String>,
    pub(crate) admin_ids: Vec<String>,
    /// Vrai dès qu'**une** compétition de l'espace interdit les matchs hors
    /// calendrier (carte 550). Porté par `PageBase` et non passé à `full_page` :
    /// sept appelants s'en servent, et la lecture est déjà faite ici, en async.
    ///
    /// La règle est celle de l'espace et non celle de cette compétition, parce
    /// que le bouton mène à `new_match_report(space_id)` — un formulaire où le
    /// coach choisit lui-même la compétition.
    pub(crate) hors_calendrier_interdit: bool,
}

pub(crate) async fn load_page_base(
    cid: &CompetitionId,
    sid: &SeasonId,
    state: &AppState,
    competition_id: &str,
    space_id: &str,
) -> Result<PageBase, Response> {
    let comp_repo = state.competitions.competition_repository.as_ref();
    let season_repo = state.competitions.season_repository.as_ref();

    let (base_info, season_info) = tokio::join!(
        comp_repo.find_base_info(cid),
        season_repo.find_base_info(sid),
    );

    let base = match base_info {
        Ok(Some(b)) => b,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("competition_detail find_base_info {competition_id}: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };

    let season_name = match season_info {
        Ok(Some(s)) => s.name,
        Ok(None) => String::new(),
        Err(e) => {
            tracing::warn!("competition_detail find_base_info season {sid}: {e}");
            String::new()
        }
    };
    let competition_initials = initials(&base.name);

    let competition_logo = base
        .logo
        .map(|url| cloudinary::transform(&url, "c_fill,w_200,h_200,q_auto,f_auto"));

    Ok(PageBase {
        competition_name: base.name,
        competition_logo,
        competition_initials,
        season_name,
        admin_names: base.admin_names,
        admin_ids: base.admin_ids,
        hors_calendrier_interdit: state.hors_calendrier.un_espace_interdit(space_id).await,
    })
}

/// Traduit l'`AuthSession` et la `PageBase` en les termes du service d'accès.
///
/// Un visiteur anonyme, ou un `space_id` que le chemin a mal formé, n'ouvre
/// rien : dans les deux cas la question ne se pose pas, et la réponse est non.
async fn peut_administrer_cette_competition(
    auth_session: &AuthSession,
    space_id: &str,
    pb: &PageBase,
    state: &AppState,
) -> bool {
    let Some(user) = auth_session.user.as_ref() else {
        return false;
    };
    let Ok(space) = SpaceId::try_new(space_id) else {
        return false;
    };
    let admins = AdminsDeLaCompetition {
        ids: &pb.admin_ids,
        names: &pb.admin_names,
    };
    let nom = user.coach_name.clone().into_inner();
    let membres = state.competitions.space_member_port.as_ref();
    peut_administrer(&user.id, &nom, &space, admins, membres).await
}

/// Le droit d'administrer est **calculé ici**, jamais reçu en paramètre.
///
/// Il l'était, et six des sept appelants passaient `false` en dur — le bouton
/// d'administration disparaissait dès qu'on entrait par un onglet, y compris
/// pour qui l'administrait (carte 544). Le rendre inexprimable par la
/// signature confie le verrou au compilateur : un huitième onglet ajouté
/// demain naîtra gardé.
pub(crate) async fn full_page(
    pb: PageBase,
    space_id: String,
    competition_id: String,
    season_id: String,
    active_tab: &'static str,
    auth_session: &AuthSession,
    state: &AppState,
    top_tds: Vec<StatRow>,
    top_casualties: Vec<StatRow>,
    flop_tds: Vec<StatRow>,
    flop_casualties: Vec<StatRow>,
) -> Response {
    let is_admin = peut_administrer_cette_competition(auth_session, &space_id, &pb, state).await;
    CompetitionDetailTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        competition_name: pb.competition_name,
        competition_logo: pb.competition_logo,
        competition_initials: pb.competition_initials,
        season_name: pb.season_name,
        admin_names: pb.admin_names,
        hors_calendrier_interdit: pb.hors_calendrier_interdit,
        is_admin,
        active_tab,
        top_tds,
        top_casualties,
        flop_tds,
        flop_casualties,
    }
    .into_response()
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_competition_detail(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let pb = match load_page_base(&cid, &sid, &state, &competition_id, &space_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };

    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "standings",
        &auth_session,
        &state,
         vec![],
        vec![],
        vec![],
        vec![],
    )
    .await
}

pub async fn get_tab_standings(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !crate::web::htmx::veut_la_page_entiere(&headers) {
        return StandingsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
        }
        .into_response();
    }
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let pb = match load_page_base(&cid, &sid, &state, &competition_id, &space_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "standings",
        &auth_session,
        &state,
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .await
}

pub async fn get_tab_detailed_standings(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !crate::web::htmx::veut_la_page_entiere(&headers) {
        return DetailedStandingsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
        }
        .into_response();
    }
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let pb = match load_page_base(&cid, &sid, &state, &competition_id, &space_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "detailed-standings",
        &auth_session,
        &state,
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .await
}

pub async fn get_tab_teams(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !crate::web::htmx::veut_la_page_entiere(&headers) {
        return TeamsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            season_id,
        }
        .into_response();
    }
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let pb = match load_page_base(&cid, &sid, &state, &competition_id, &space_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "teams",
        &auth_session,
        &state,
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .await
}

pub async fn get_tab_stats(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !crate::web::htmx::veut_la_page_entiere(&headers) {
        return StatsTabTemplate {
            top_tds: mock_top_tds(),
            top_casualties: mock_top_casualties(),
            flop_tds: mock_flop_tds(),
            flop_casualties: mock_flop_casualties(),
        }
        .into_response();
    }
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let pb = match load_page_base(&cid, &sid, &state, &competition_id, &space_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "stats",
        &auth_session,
        &state,
        mock_top_tds(),
        mock_top_casualties(),
        mock_flop_tds(),
        mock_flop_casualties(),
    )
    .await
}
