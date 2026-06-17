use crate::app::players::routes::Routes as PlayerRoutes;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::app::teams::domain::team::{GamePhase, ParticipationStatus, Team};
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────

pub struct StaffLineVm {
    pub label: String,
    pub quantity: u8,
    pub unit_price: u32,
    pub total_price: u32,
}

pub struct StaffVm {
    pub lines: Vec<StaffLineVm>,
    pub grand_total: u32,
}

impl StaffVm {
    fn from(team: &Team, reroll_price_kpo: u32) -> Self {
        let mut lines = vec![
            StaffLineVm {
                label: "Relances".into(),
                quantity: team.rerolls.0,
                unit_price: reroll_price_kpo,
                total_price: team.rerolls.0 as u32 * reroll_price_kpo,
            },
            StaffLineVm {
                label: "Apothicaire".into(),
                quantity: team.apothecaries.0,
                unit_price: 50,
                total_price: team.apothecaries.0 as u32 * 50,
            },
            StaffLineVm {
                label: "Assistants entraîneurs".into(),
                quantity: team.assistants.0,
                unit_price: 10,
                total_price: team.assistants.0 as u32 * 10,
            },
            StaffLineVm {
                label: "Pom-pom girls".into(),
                quantity: team.cheerleaders.0,
                unit_price: 10,
                total_price: team.cheerleaders.0 as u32 * 10,
            },
        ];

        let grand_total = lines.iter().map(|l| l.total_price).sum();
        Self { lines, grand_total }
    }
}

pub struct TeamDetailVm {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub logo_url: Option<String>,
    pub roster_name: String,
    pub roster_initials: String,
    pub roster_logo_url: Option<String>,
    pub coach_name: String,
    pub dedicated_fans: u8,
    pub treasury_kpo: u32,
    pub team_value_kpo: u32,
    pub competition_name: Option<String>,
    pub season_name: Option<String>,
    pub status_label: String,
    pub status_css_class: String,
    pub players_widget_url: String,
    pub staff: StaffVm,
}

impl TeamDetailVm {
    fn from(team: &Team, space_id: &str, ref_repo: &dyn IReferenceRepository) -> Self {
        let (status_label, status_css_class) = status_display(team);
        let roster_initials = team
            .roster_name
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();

        let ref_roster = ref_repo.find_team_by_uid(&team.roster_id);

        let roster_logo_url = ref_roster.and_then(|t| t.logo.as_deref()).map(|url| {
            crate::app::shared_kernel::cloudinary::transform(
                url,
                "c_fill,w_120,h_120,q_auto,f_auto",
            )
        });

        let reroll_price_kpo = ref_roster.map(|t| t.reroll_cost / 1000).unwrap_or(50);

        let logo_url = team.logo_url.as_deref().map(|url| {
            crate::app::shared_kernel::cloudinary::transform(
                url,
                "c_fill,w_120,h_120,q_auto,f_auto",
            )
        });

        Self {
            id: team.id.clone(),
            name: team.name.clone(),
            initials: team.initials.clone(),
            logo_url,
            roster_name: team.roster_name.clone(),
            roster_initials,
            roster_logo_url,
            coach_name: team.coach_name.clone(),
            dedicated_fans: team.dedicated_fans,
            treasury_kpo: team.treasury.0,
            team_value_kpo: team.team_value.0,
            competition_name: team.competition_name.clone(),
            season_name: team.season_name.clone(),
            status_label,
            status_css_class,
            players_widget_url: PlayerRoutes.players_by_team_widget(space_id, &team.id),
            staff: StaffVm::from(team, reroll_price_kpo),
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
    let ref_repo = state.references.repository.as_ref();

    TeamDetailTemplate {
        web_routes: Default::default(),
        vm: TeamDetailVm::from(&team, &space_id, ref_repo),
        back_url,
    }
    .into_response()
}
