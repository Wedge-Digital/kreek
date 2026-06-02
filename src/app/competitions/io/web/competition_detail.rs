use crate::app::competitions::routes::Routes;
use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

use crate::app::shared_kernel::cloudinary;

// ── Mock data structs ─────────────────────────────────────────────────────────

pub struct StandingRow {
    pub rank: u32,
    pub name: String,
    pub played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub points: u32,
}

pub struct MatchEntry {
    pub home: String,
    pub home_roster: String,
    pub home_coach: String,
    pub away: String,
    pub away_roster: String,
    pub away_coach: String,
    pub home_score: u32,
    pub away_score: u32,
    pub home_casualties: u32,
    pub away_casualties: u32,
    pub date: String,
}

pub struct Journee {
    pub label: String,
    pub matches: Vec<MatchEntry>,
}

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

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn mock_standings() -> Vec<StandingRow> {
    vec![
        StandingRow {
            rank: 1,
            name: "Les Guerriers du Nord".into(),
            played: 8,
            wins: 7,
            draws: 0,
            losses: 1,
            points: 21,
        },
        StandingRow {
            rank: 2,
            name: "Waaagh! FC".into(),
            played: 8,
            wins: 6,
            draws: 1,
            losses: 1,
            points: 19,
        },
        StandingRow {
            rank: 3,
            name: "Chaos United".into(),
            played: 8,
            wins: 5,
            draws: 2,
            losses: 1,
            points: 17,
        },
        StandingRow {
            rank: 4,
            name: "Elfes Sylvains SC".into(),
            played: 8,
            wins: 5,
            draws: 0,
            losses: 3,
            points: 15,
        },
        StandingRow {
            rank: 5,
            name: "Nains de Fer".into(),
            played: 8,
            wins: 4,
            draws: 1,
            losses: 3,
            points: 13,
        },
        StandingRow {
            rank: 6,
            name: "Skavens du Sous-sol".into(),
            played: 8,
            wins: 3,
            draws: 2,
            losses: 3,
            points: 11,
        },
        StandingRow {
            rank: 7,
            name: "Humains Ordinaires".into(),
            played: 8,
            wins: 3,
            draws: 0,
            losses: 5,
            points: 9,
        },
        StandingRow {
            rank: 8,
            name: "Nécromants XI".into(),
            played: 8,
            wins: 2,
            draws: 1,
            losses: 5,
            points: 7,
        },
        StandingRow {
            rank: 9,
            name: "Orques Sauvages".into(),
            played: 8,
            wins: 1,
            draws: 0,
            losses: 7,
            points: 3,
        },
        StandingRow {
            rank: 10,
            name: "Halflings United".into(),
            played: 8,
            wins: 0,
            draws: 1,
            losses: 7,
            points: 1,
        },
    ]
}

fn mock_journees() -> Vec<Journee> {
    vec![
        Journee {
            label: "Journée 8".into(),
            matches: vec![
                MatchEntry {
                    home: "Les Guerriers du Nord".into(),
                    home_roster: "Nordiques".into(),
                    home_coach: "CoachAlpha".into(),
                    away: "Chaos United".into(),
                    away_roster: "Chaos".into(),
                    away_coach: "ChaosBoss".into(),
                    home_score: 2,
                    away_score: 1,
                    home_casualties: 3,
                    away_casualties: 1,
                    date: "18 mai".into(),
                },
                MatchEntry {
                    home: "Waaagh! FC".into(),
                    home_roster: "Orques".into(),
                    home_coach: "WaaghMaster".into(),
                    away: "Elfes Sylvains SC".into(),
                    away_roster: "Elfes Sylvains".into(),
                    away_coach: "ElveRunner".into(),
                    home_score: 3,
                    away_score: 0,
                    home_casualties: 5,
                    away_casualties: 0,
                    date: "18 mai".into(),
                },
                MatchEntry {
                    home: "Nains de Fer".into(),
                    home_roster: "Nains".into(),
                    home_coach: "IronBeard".into(),
                    away: "Skavens du Sous-sol".into(),
                    away_roster: "Skavens".into(),
                    away_coach: "SkavRunner".into(),
                    home_score: 1,
                    away_score: 1,
                    home_casualties: 2,
                    away_casualties: 2,
                    date: "19 mai".into(),
                },
                MatchEntry {
                    home: "Humains Ordinaires".into(),
                    home_roster: "Humains".into(),
                    home_coach: "HumanCoach".into(),
                    away: "Halflings United".into(),
                    away_roster: "Halflings".into(),
                    away_coach: "TinyCoach".into(),
                    home_score: 2,
                    away_score: 0,
                    home_casualties: 1,
                    away_casualties: 4,
                    date: "19 mai".into(),
                },
            ],
        },
        Journee {
            label: "Journée 7".into(),
            matches: vec![
                MatchEntry {
                    home: "Chaos United".into(),
                    home_roster: "Chaos".into(),
                    home_coach: "ChaosBoss".into(),
                    away: "Waaagh! FC".into(),
                    away_roster: "Orques".into(),
                    away_coach: "WaaghMaster".into(),
                    home_score: 1,
                    away_score: 2,
                    home_casualties: 4,
                    away_casualties: 2,
                    date: "11 mai".into(),
                },
                MatchEntry {
                    home: "Elfes Sylvains SC".into(),
                    home_roster: "Elfes Sylvains".into(),
                    home_coach: "ElveRunner".into(),
                    away: "Les Guerriers du Nord".into(),
                    away_roster: "Nordiques".into(),
                    away_coach: "CoachAlpha".into(),
                    home_score: 0,
                    away_score: 3,
                    home_casualties: 0,
                    away_casualties: 1,
                    date: "11 mai".into(),
                },
                MatchEntry {
                    home: "Nécromants XI".into(),
                    home_roster: "Nécromants".into(),
                    home_coach: "NecroMind".into(),
                    away: "Nains de Fer".into(),
                    away_roster: "Nains".into(),
                    away_coach: "IronBeard".into(),
                    home_score: 0,
                    away_score: 2,
                    home_casualties: 1,
                    away_casualties: 3,
                    date: "12 mai".into(),
                },
                MatchEntry {
                    home: "Orques Sauvages".into(),
                    home_roster: "Orques Sauvages".into(),
                    home_coach: "GreenFist".into(),
                    away: "Humains Ordinaires".into(),
                    away_roster: "Humains".into(),
                    away_coach: "HumanCoach".into(),
                    home_score: 1,
                    away_score: 1,
                    home_casualties: 3,
                    away_casualties: 1,
                    date: "12 mai".into(),
                },
            ],
        },
    ]
}

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
    pub web_routes: WebRoutes,
    pub competition_routes: Routes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub competition_name: String,
    pub competition_logo: Option<String>,
    pub competition_initials: String,
    pub season_name: String,
    pub admin_names: Vec<String>,
    pub active_tab: &'static str,
    // tab content (only one is populated per request)
    pub standings: Vec<StandingRow>,
    pub journees: Vec<Journee>,
    pub teams: Vec<TeamCard>,
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
    pub standings: Vec<StandingRow>,
}

#[derive(Template)]
#[template(path = "competition-tab-matches.html")]
pub struct MatchesTabTemplate {
    pub journees: Vec<Journee>,
}

#[derive(Template)]
#[template(path = "competition-tab-teams.html")]
pub struct TeamsTabTemplate {
    pub teams: Vec<TeamCard>,
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
impl IntoResponse for MatchesTabTemplate {
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

struct PageBase {
    competition_name: String,
    competition_logo: Option<String>,
    competition_initials: String,
    season_name: String,
    admin_names: Vec<String>,
}

async fn load_page_base(
    cid: &CompetitionId,
    sid: &SeasonId,
    state: &AppState,
    competition_id: &str,
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
    })
}

fn full_page(
    pb: PageBase,
    space_id: String,
    competition_id: String,
    season_id: String,
    active_tab: &'static str,
    standings: Vec<StandingRow>,
    journees: Vec<Journee>,
    teams: Vec<TeamCard>,
    top_tds: Vec<StatRow>,
    top_casualties: Vec<StatRow>,
    flop_tds: Vec<StatRow>,
    flop_casualties: Vec<StatRow>,
) -> Response {
    CompetitionDetailTemplate {
        web_routes: WebRoutes,
        competition_routes: Routes,
        space_id,
        competition_id,
        season_id,
        competition_name: pb.competition_name,
        competition_logo: pb.competition_logo,
        competition_initials: pb.competition_initials,
        season_name: pb.season_name,
        admin_names: pb.admin_names,
        active_tab,
        standings,
        journees,
        teams,
        top_tds,
        top_casualties,
        flop_tds,
        flop_casualties,
    }
    .into_response()
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_competition_detail(
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

    let pb = match load_page_base(&cid, &sid, &state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };

    CompetitionDetailTemplate {
        web_routes: WebRoutes,
        competition_routes: Routes,
        space_id,
        competition_id,
        season_id,
        competition_name: pb.competition_name,
        competition_logo: pb.competition_logo,
        competition_initials: pb.competition_initials,
        season_name: pb.season_name,
        admin_names: pb.admin_names,
        active_tab: "standings",
        standings: mock_standings(),
        journees: vec![],
        teams: vec![],
        top_tds: vec![],
        top_casualties: vec![],
        flop_tds: vec![],
        flop_casualties: vec![],
    }
    .into_response()
}

pub async fn get_tab_standings(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        return StandingsTabTemplate {
            standings: mock_standings(),
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
    let pb = match load_page_base(&cid, &sid, &state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "standings",
        mock_standings(),
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
}

pub async fn get_tab_matches(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        return MatchesTabTemplate {
            journees: mock_journees(),
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
    let pb = match load_page_base(&cid, &sid, &state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "matches",
        vec![],
        mock_journees(),
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
}

pub async fn get_tab_teams(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        return TeamsTabTemplate {
            teams: mock_teams(),
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
    let pb = match load_page_base(&cid, &sid, &state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "teams",
        vec![],
        vec![],
        mock_teams(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
}

pub async fn get_tab_stats(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
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
    let pb = match load_page_base(&cid, &sid, &state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "stats",
        vec![],
        vec![],
        vec![],
        mock_top_tds(),
        mock_top_casualties(),
        mock_flop_tds(),
        mock_flop_casualties(),
    )
}
