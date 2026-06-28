use crate::app::competitions::domain::competition_structure::ScheduledDate;
use crate::app::competitions::domain::match_day::MatchDayType;
use crate::app::shared_kernel::date_string::DateString;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::SeasonId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use std::collections::HashMap;

// ── Round sidebar ────────────────────────────────────────────────────────────

pub struct RoundItemVm {
    pub round_id: String,
    pub name: String,
    pub date_label: String,
    pub day_type: String,
    pub status: String,
}

#[derive(Template)]
#[template(path = "admin/widgets/schedule-sidebar.html")]
pub struct RoundSidebarTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub rounds: Vec<RoundItemVm>,
}

impl IntoResponse for RoundSidebarTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("schedule sidebar render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

fn date_label_for(day_type: &MatchDayType, date_start: &Option<DateString>, date_end: &Option<DateString>) -> String {
    match day_type {
        MatchDayType::FixedDate => date_start.as_ref().map(|d| d.to_string()).unwrap_or_default(),
        MatchDayType::TimeFrame => {
            let s = date_start.as_ref().map(|d| d.to_string()).unwrap_or_default();
            let e = date_end.as_ref().map(|d| d.to_string()).unwrap_or_default();
            if s.is_empty() && e.is_empty() {
                String::new()
            } else {
                format!("{s} → {e}")
            }
        }
        MatchDayType::Rest => String::new(),
    }
}

pub async fn schedule_sidebar_widget(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    // Sync match days from structure if needed
    let structure = state
        .competitions
        .season_repository
        .find_structure(&sid)
        .await
        .ok()
        .flatten();

    if let Some(ref s) = structure {
        let entries: Vec<(String, String, String, Option<String>, Option<String>)> = s
            .schedule
            .scheduled_dates
            .iter()
            .map(|sd| match sd {
                ScheduledDate::FixedDate {
                    name,
                    multiplexe_date,
                } => (
                    ulid::Ulid::new().to_string(),
                    name.as_ref().to_string(),
                    "fixed_date".to_string(),
                    Some(multiplexe_date.as_ref().to_string()),
                    Some(multiplexe_date.as_ref().to_string()),
                ),
                ScheduledDate::TimeFrame {
                    name,
                    start_date,
                    end_date,
                } => (
                    ulid::Ulid::new().to_string(),
                    name.as_ref().to_string(),
                    "time_frame".to_string(),
                    Some(start_date.as_ref().to_string()),
                    Some(end_date.as_ref().to_string()),
                ),
            })
            .collect();

        if !entries.is_empty() {
            if let Err(e) = state
                .competitions
                .match_day_repository
                .ensure_match_days_from_structure(&season_id, &entries)
                .await
            {
                tracing::error!("ensure_match_days: {e}");
            }
        }
    }

    let match_days = match state
        .competitions
        .match_day_repository
        .find_by_season(&season_id)
        .await
    {
        Ok(days) => days,
        Err(e) => {
            tracing::error!("schedule_sidebar match_days: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let rounds = match_days
        .into_iter()
        .map(|d| {
            let day_type_str = match d.day_type {
                MatchDayType::Rest => "rest".to_string(),
                _ => "match".to_string(),
            };
            let dl = date_label_for(&d.day_type, &d.date_start, &d.date_end);
            RoundItemVm {
                round_id: d.id.to_string(),
                name: d.name.into_inner(),
                date_label: dl,
                day_type: day_type_str,
                status: "upcoming".to_string(),
            }
        })
        .collect();

    RoundSidebarTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        rounds,
    }
    .into_response()
}

// ── Round detail ─────────────────────────────────────────────────────────────

pub struct FixtureVm {
    pub fixture_id: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub group_name: String,
}

pub struct TeamOptionVm {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
}

pub struct RoundDetailVm {
    pub round_id: String,
    pub name: String,
    pub day_type: String,
    pub date_start: String,
    pub date_end: String,
    pub fixtures: Vec<FixtureVm>,
    pub team_options: Vec<TeamOptionVm>,
}

#[derive(Template)]
#[template(path = "admin/widgets/schedule-round-detail.html")]
pub struct RoundDetailTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub detail: RoundDetailVm,
}

impl IntoResponse for RoundDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("round detail render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Deserialize)]
pub struct RoundDetailQuery {
    pub round_id: Option<String>,
}

pub async fn schedule_round_detail_widget(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Query(query): Query<RoundDetailQuery>,
) -> impl IntoResponse {
    let round_id = match query.round_id {
        Some(ref id) if !id.is_empty() => id.clone(),
        _ => {
            return Html(
                r#"<div class="round-detail-empty"><div class="empty-state-icon">📅</div><div class="empty-state-text">Sélectionnez une journée dans la liste.</div></div>"#.to_string()
            ).into_response();
        }
    };

    let match_day = match state
        .competitions
        .match_day_repository
        .find_by_id(&round_id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("round_detail find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Load enrolled teams for team name resolution + TomSelect options
    let enrolled = match state
        .competitions
        .team_info_port
        .find_enrolled_teams(&season_id)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("round_detail enrolled: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let team_map: HashMap<&str, &crate::app::competitions::ports::TeamInfoDto> =
        enrolled.iter().map(|t| (t.team_id.as_str(), t)).collect();

    // Load groups for group_name resolution
    let groups = state
        .competitions
        .group_repository
        .find_groups(&season_id)
        .await
        .unwrap_or_default();

    let team_to_group: HashMap<&str, &str> = groups
        .iter()
        .flat_map(|g| {
            g.team_ids
                .iter()
                .map(move |tid| (tid.as_str(), g.group_name.as_str()))
        })
        .collect();

    let fixtures = match_day
        .pairings
        .iter()
        .map(|p| {
            let home_id = p.home_team_id.to_string();
            let away_id = p.away_team_id.to_string();
            let home_name = team_map
                .get(home_id.as_str())
                .map(|t| t.team_name.clone())
                .unwrap_or_else(|| home_id.clone());
            let away_name = team_map
                .get(away_id.as_str())
                .map(|t| t.team_name.clone())
                .unwrap_or(away_id);
            let gn = team_to_group
                .get(home_id.as_str())
                .copied()
                .unwrap_or("");
            FixtureVm {
                fixture_id: p.id.to_string(),
                home_team_name: home_name,
                away_team_name: away_name,
                group_name: gn.to_string(),
            }
        })
        .collect();

    let team_options = enrolled
        .iter()
        .map(|t| TeamOptionVm {
            team_id: t.team_id.clone(),
            team_name: t.team_name.clone(),
            coach_name: t.coach_name.clone(),
            roster_name: t.roster_name.clone(),
        })
        .collect();

    let day_type_str = match match_day.day_type {
        MatchDayType::Rest => "rest".to_string(),
        MatchDayType::FixedDate => "fixed_date".to_string(),
        MatchDayType::TimeFrame => "time_frame".to_string(),
    };

    let detail = RoundDetailVm {
        round_id: match_day.id.to_string(),
        name: match_day.name.into_inner(),
        day_type: day_type_str,
        date_start: match_day.date_start.unwrap_or_default().into_inner(),
        date_end: match_day.date_end.unwrap_or_default().into_inner(),
        fixtures,
        team_options,
    };

    RoundDetailTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        detail,
    }
    .into_response()
}
