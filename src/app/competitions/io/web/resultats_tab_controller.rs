use crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto;
use crate::app::competitions::io::web::competition_detail::{load_page_base, full_page};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct TabCursorQuery {
    pub cursor: Option<i32>,
}

pub struct MatchResultatVm {
    pub home_name: String,
    pub home_roster: String,
    pub home_coach: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_roster: String,
    pub away_coach: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
    pub is_in_progress: bool,
    pub is_completed: bool,
    pub home_score: Option<u32>,
    pub away_score: Option<u32>,
    pub home_cas: Option<u32>,
    pub away_cas: Option<u32>,
    pub report_url: Option<String>,
}

pub struct JourneeResultatsVm {
    pub label: String,
    pub matches: Vec<MatchResultatVm>,
}

#[derive(Template)]
#[template(path = "competition-tab-resultats.html")]
pub struct ResultatsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeResultatsVm>,
    pub next_cursor: Option<i32>,
    pub is_initial: bool,
}

impl IntoResponse for ResultatsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_resultats_tab(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<TabCursorQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let rows = match load_resultats(&state, &season_id, query.cursor).await {
        Ok(r) => r,
        Err(r) => return r,
    };

    let (journees, next_cursor) = build_journees(rows, 3);
    let is_htmx = headers.contains_key("hx-request");

    if is_htmx {
        return ResultatsTabTemplate {
            app_routes: AppRoutes::default(),
            space_id,
            competition_id,
            season_id,
            journees,
            next_cursor,
            is_initial: query.cursor.is_none(),
        }
        .into_response();
    }

    render_full_page(space_id, competition_id, season_id, journees, next_cursor, &state).await
}

async fn load_resultats(
    state: &AppState,
    season_id: &str,
    cursor: Option<i32>,
) -> Result<Vec<PairingDisplayDto>, Response> {
    state
        .competitions
        .match_day_repository
        .list_resultats(season_id, cursor, 3)
        .await
        .map_err(|e| {
            tracing::error!("resultats_tab: list_resultats: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}

fn build_journees(
    rows: Vec<PairingDisplayDto>,
    max_rounds: usize,
) -> (Vec<JourneeResultatsVm>, Option<i32>) {
    let mut by_round: BTreeMap<i32, (String, Vec<MatchResultatVm>)> = BTreeMap::new();
    for row in rows {
        let entry = by_round
            .entry(row.round_position)
            .or_insert_with(|| (row.round_name.clone(), Vec::new()));
        entry.1.push(to_resultat_vm(row));
    }

    let mut journees: Vec<(i32, JourneeResultatsVm)> = by_round
        .into_iter()
        .map(|(pos, (label, matches))| (pos, JourneeResultatsVm { label, matches }))
        .collect();

    journees.sort_by(|a, b| b.0.cmp(&a.0));
    journees.truncate(max_rounds);

    let next_cursor = if journees.len() == max_rounds {
        journees.last().map(|(pos, _)| *pos)
    } else {
        None
    };

    (journees.into_iter().map(|(_, j)| j).collect(), next_cursor)
}

fn to_resultat_vm(row: PairingDisplayDto) -> MatchResultatVm {
    let is_completed = row.match_status == "completed";
    let is_in_progress = row.match_status == "in_progress";
    MatchResultatVm {
        home_name: row.home_team_name,
        home_roster: row.home_roster_name,
        home_coach: row.home_coach_name,
        home_logo: row.home_logo_url,
        home_initials: row.home_initials,
        away_name: row.away_team_name,
        away_roster: row.away_roster_name,
        away_coach: row.away_coach_name,
        away_logo: row.away_logo_url,
        away_initials: row.away_initials,
        is_in_progress,
        is_completed,
        home_score: row.home_score.map(|v| v as u32),
        away_score: row.away_score.map(|v| v as u32),
        home_cas: row.home_casualties.map(|v| v as u32),
        away_cas: row.away_casualties.map(|v| v as u32),
        report_url: row.match_report_url,
    }
}

async fn render_full_page(
    space_id: String,
    competition_id: String,
    season_id: String,
    journees: Vec<JourneeResultatsVm>,
    next_cursor: Option<i32>,
    state: &AppState,
) -> Response {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let pb = match load_page_base(&cid, &sid, state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let _ = (journees, next_cursor);
    full_page(pb, space_id, competition_id, season_id, "resultats", false,
        vec![], vec![], vec![], vec![], vec![])
}
