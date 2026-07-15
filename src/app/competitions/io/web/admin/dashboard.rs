use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::use_cases::admin::dashboard_query;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct DashboardAlertVm {
    pub message: String,
    pub action_label: String,
    pub action_url: String,
    pub level: String,
}

pub struct DashboardStatVm {
    pub icon: String,
    pub value: String,
    pub label: String,
    pub style: String,
}

pub struct DashboardProgressVm {
    pub label: String,
    pub current: u32,
    pub total: u32,
    pub pct: u32,
    pub color: String,
}

pub struct DashboardActivityVm {
    pub icon_type: String,
    pub text: String,
    pub time: String,
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct DashboardFragmentTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub alerts: Vec<DashboardAlertVm>,
    pub stats: Vec<DashboardStatVm>,
    pub progress: Vec<DashboardProgressVm>,
    pub activity: Vec<DashboardActivityVm>,
}

impl IntoResponse for DashboardFragmentTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("dashboard fragment render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

fn pct(current: u32, total: u32) -> u32 {
    if total == 0 { 0 } else { (current * 100) / total }
}

pub fn build_dashboard_fragment(
    summary: &dashboard_query::DashboardSummary,
    app_routes: &AppRoutes,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
) -> DashboardFragmentTemplate {
    let mut alerts = Vec::new();

    if summary.pending_count > 0 {
        alerts.push(DashboardAlertVm {
            message: format!(
                "<strong>{} inscription{}</strong> en attente de validation.",
                summary.pending_count,
                if summary.pending_count > 1 { "s" } else { "" }
            ),
            action_label: "Gérer →".into(),
            action_url: "#".into(),
            level: "warn".into(),
        });
    }

    if summary.rounds_validated < summary.rounds_total && summary.matches_played > 0 {
        alerts.push(DashboardAlertVm {
            message: "<strong>Des résultats</strong> sont en attente de validation.".into(),
            action_label: "Valider →".into(),
            action_url: "#".into(),
            level: "info".into(),
        });
    }

    let enrolled = summary.enrolled_count as u32;
    let pending = summary.pending_count as u32;
    let played = summary.matches_played as u32;
    let total_matches = summary.matches_total as u32;
    let validated = summary.rounds_validated as u32;
    let total_rounds = summary.rounds_total as u32;
    let remaining = total_matches.saturating_sub(played);

    let stats = vec![
        DashboardStatVm {
            icon: "👥".into(),
            value: format!("{}", enrolled),
            label: "Équipes inscrites".into(),
            style: "default".into(),
        },
        DashboardStatVm {
            icon: "⏳".into(),
            value: format!("{}", pending),
            label: "En attente".into(),
            style: if pending > 0 { "warn".into() } else { "default".into() },
        },
        DashboardStatVm {
            icon: "⚔️".into(),
            value: format!("{}", played),
            label: "Matchs joués".into(),
            style: "default".into(),
        },
        DashboardStatVm {
            icon: "📅".into(),
            value: format!("{}", remaining),
            label: "Matchs restants".into(),
            style: "default".into(),
        },
        DashboardStatVm {
            icon: "✅".into(),
            value: format!("{} / {}", validated, total_rounds),
            label: "Journées validées".into(),
            style: if validated == total_rounds && total_rounds > 0 { "ok".into() } else { "default".into() },
        },
    ];

    let max = summary.max_participants.unwrap_or(0);

    let progress = vec![
        DashboardProgressVm {
            label: "Inscriptions".into(),
            current: enrolled,
            total: if max > 0 { max } else { enrolled },
            pct: if max > 0 { pct(enrolled, max) } else { 100 },
            color: "green".into(),
        },
        DashboardProgressVm {
            label: "Matchs joués".into(),
            current: played,
            total: total_matches,
            pct: pct(played, total_matches),
            color: "blue".into(),
        },
        DashboardProgressVm {
            label: "Journées validées".into(),
            current: validated,
            total: total_rounds,
            pct: pct(validated, total_rounds),
            color: "orange".into(),
        },
    ];

    let activity: Vec<DashboardActivityVm> = summary
        .recent_activity
        .iter()
        .map(|a| {
            let icon_type = match a.kind {
                dashboard_query::ActivityKind::Enrollment => "enroll",
                dashboard_query::ActivityKind::MatchResult => "match",
                dashboard_query::ActivityKind::Validation => "enroll",
                dashboard_query::ActivityKind::RestDay => "warn",
            };
            DashboardActivityVm {
                icon_type: icon_type.into(),
                text: a.description.clone(),
                time: a.occurred_at.clone(),
            }
        })
        .collect();

    DashboardFragmentTemplate {
        app_routes: *app_routes,
        space_id: space_id.to_string(),
        competition_id: competition_id.to_string(),
        season_id: season_id.to_string(),
        alerts,
        stats,
        progress,
        activity,
    }
}

pub async fn dashboard_fragment(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Err(resp) = require_admin_access(&auth_session, &space_id, &competition_id, &state).await {
        return resp;
    }

    let comp_id = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let season_entity_id = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let summary = match dashboard_query::execute(
        &comp_id,
        &season_entity_id,
        state.competitions.competition_repository.as_ref(),
        state.competitions.season_repository.as_ref(),
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("dashboard_query error: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let app_routes = AppRoutes::default();
    build_dashboard_fragment(&summary, &app_routes, &space_id, &competition_id, &season_id)
        .into_response()
}
