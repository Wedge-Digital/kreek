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

pub struct MatchCalendrierVm {
    pub home_name: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
}

pub struct JourneeCalendrierVm {
    pub label: String,
    pub date_range: String,
    pub match_count: usize,
    pub matches: Vec<MatchCalendrierVm>,
}

#[derive(Template)]
#[template(path = "competition-tab-calendrier.html")]
pub struct CalendrierTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeCalendrierVm>,
    pub next_cursor: Option<i32>,
    pub is_initial: bool,
}

impl IntoResponse for CalendrierTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_calendrier_tab(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<TabCursorQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let rows = match load_calendrier(&state, &season_id, query.cursor).await {
        Ok(r) => r,
        Err(r) => return r,
    };

    let (journees, next_cursor) = build_journees(rows, 3);
    let is_htmx = headers.contains_key("hx-request");

    if is_htmx {
        return CalendrierTabTemplate {
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

async fn load_calendrier(
    state: &AppState,
    season_id: &str,
    cursor: Option<i32>,
) -> Result<Vec<PairingDisplayDto>, Response> {
    state
        .competitions
        .match_day_repository
        .list_calendrier(season_id, cursor, 3)
        .await
        .map_err(|e| {
            tracing::error!("calendrier_tab: list_calendrier: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}

fn build_journees(
    rows: Vec<PairingDisplayDto>,
    max_rounds: usize,
) -> (Vec<JourneeCalendrierVm>, Option<i32>) {
    let mut by_round: BTreeMap<i32, (String, String, String, Vec<MatchCalendrierVm>)> =
        BTreeMap::new();
    for row in rows {
        let date_range = format_date_range(&row.round_day_type, row.round_date_start.as_deref(), row.round_date_end.as_deref());
        let entry = by_round
            .entry(row.round_position)
            .or_insert_with(|| (row.round_name.clone(), date_range, row.round_day_type.clone(), Vec::new()));
        entry.3.push(to_calendrier_vm(row));
    }

    let mut journees: Vec<(i32, JourneeCalendrierVm)> = by_round
        .into_iter()
        .map(|(pos, (label, date_range, _, matches))| {
            let match_count = matches.len();
            (pos, JourneeCalendrierVm { label, date_range, match_count, matches })
        })
        .collect();

    journees.sort_by_key(|(pos, _)| *pos);
    journees.truncate(max_rounds);

    let next_cursor = if journees.len() == max_rounds {
        journees.last().map(|(pos, _)| *pos)
    } else {
        None
    };

    (journees.into_iter().map(|(_, j)| j).collect(), next_cursor)
}

fn format_date_range(day_type: &str, start: Option<&str>, end: Option<&str>) -> String {
    match day_type {
        "fixed_date" => start.unwrap_or("").to_string(),
        "time_frame" => match (start, end) {
            (Some(s), Some(e)) => format!("{} – {}", s, e),
            (Some(s), None) => s.to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

fn to_calendrier_vm(row: PairingDisplayDto) -> MatchCalendrierVm {
    MatchCalendrierVm {
        home_name: row.home_team_name,
        home_logo: row.home_logo_url,
        home_initials: row.home_initials,
        away_name: row.away_team_name,
        away_logo: row.away_logo_url,
        away_initials: row.away_initials,
    }
}

async fn render_full_page(
    space_id: String,
    competition_id: String,
    season_id: String,
    journees: Vec<JourneeCalendrierVm>,
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
    full_page(pb, space_id, competition_id, season_id, "calendrier", false,
        vec![], vec![], vec![], vec![], vec![], vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_fixed_date() {
        assert_eq!(format_date_range("fixed_date", Some("12 juin"), None), "12 juin");
    }

    #[test]
    fn format_time_frame_with_end() {
        assert_eq!(
            format_date_range("time_frame", Some("10 juin"), Some("12 juin")),
            "10 juin – 12 juin"
        );
    }

    #[test]
    fn format_time_frame_without_end() {
        assert_eq!(format_date_range("time_frame", Some("10 juin"), None), "10 juin");
    }

    #[test]
    fn format_rest_returns_empty() {
        assert_eq!(format_date_range("rest", None, None), "");
    }
}
