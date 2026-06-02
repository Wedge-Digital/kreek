use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::app::teams::domain::team::{GamePhase, ParticipationStatus, Team};
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────

pub struct TeamDetailVm {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub roster_name: String,
    pub roster_initials: String,
    pub coach_name: String,
    pub dedicated_fans: u8,
    pub treasury_kpo: u32,
    pub team_value_kpo: u32,
    pub competition_name: Option<String>,
    pub season_name: Option<String>,
    pub status_label: String,
    pub status_css_class: String,
    pub players_widget_url: String,
}

impl TeamDetailVm {
    fn from(team: &Team, space_id: &str) -> Self {
        let (status_label, status_css_class) = status_display(team);
        let roster_initials = team
            .roster_name
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();

        Self {
            id: team.id.clone(),
            name: team.name.clone(),
            initials: team.initials.clone(),
            roster_name: team.roster_name.clone(),
            roster_initials,
            coach_name: team.coach_name.clone(),
            dedicated_fans: team.dedicated_fans,
            treasury_kpo: team.treasury.0,
            team_value_kpo: team.team_value.0,
            competition_name: team.competition_name.clone(),
            season_name: team.season_name.clone(),
            status_label,
            status_css_class,
            players_widget_url: format!("/app/{space_id}/players/by-team/{}/widget", &team.id),
        }
    }
}

fn status_display(team: &Team) -> (String, String) {
    match &team.participation_status {
        ParticipationStatus::Dismissed => ("Renvoyée".into(), "dismissed".into()),
        ParticipationStatus::PendingEnrollment => {
            ("En attente d'inscription".into(), "pending".into())
        }
        ParticipationStatus::Enrolled => match &team.game_phase {
            Some(GamePhase::ReadyToPlay) => ("Prête à jouer".into(), "ready".into()),
            Some(GamePhase::PlayerImprovement) => ("Phase d'amélioration".into(), "phase".into()),
            Some(GamePhase::Recruitment) => ("Phase de recrutement".into(), "phase".into()),
            Some(GamePhase::Dismissals) => ("Phase de renvois".into(), "phase".into()),
            Some(GamePhase::TemporaryRetirement) => ("Retraite temporaire".into(), "phase".into()),
            Some(GamePhase::OffSeason) => ("Off-season".into(), "offseason".into()),
            None => ("Inscrite".into(), "ready".into()),
        },
    }
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "teams-team-detail.html")]
pub struct TeamDetailTemplate {
    pub web_routes: WebRoutes,
    pub vm: TeamDetailVm,
    pub back_url: String,
}

impl IntoResponse for TeamDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn team_detail(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team = match state.teams.team_repository.find_by_id(&team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("team_detail find_by_id {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let back_url = TeamCreationRoutes::default().my_teams(&space_id);

    TeamDetailTemplate {
        web_routes: Default::default(),
        vm: TeamDetailVm::from(&team, &space_id),
        back_url,
    }
    .into_response()
}
