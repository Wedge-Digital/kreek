use crate::app::auth::auth_backend::AuthSession;
use crate::app::references::routes::Routes as RefRoutes;
use crate::app::team_creation::domain::roster::LeagueId;
use crate::app::team_creation::io::web::view_models::{FinalizePlayerVm, SppLogEntryVm};
use crate::app::team_creation::routes::Routes;
use crate::app::team_creation::use_cases::commands::SubmitTeamCommand;
use crate::app::team_creation::use_cases::set_league::{SetLeagueCommand, SetLeagueError};
use crate::app::team_creation::use_cases::submit_team as submit_uc;
use crate::app::shared_kernel::common_types::EntityId;
use crate::web::routes::Routes as WebRoutes;
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "finalize-team.html")]
pub struct FinalizeTeamTemplate {
    pub web_routes: WebRoutes,
    pub team_routes: Routes,
    pub ref_routes: RefRoutes,
    pub space_id: String,
    pub team_id: String,
    pub logo_url: Option<String>,
    pub team_name: String,
    pub roster_name: String,
    pub treasury: u32,
    pub spp_pool: u8,
    pub players: Vec<FinalizePlayerVm>,
    pub spp_log: Vec<SppLogEntryVm>,
}

impl IntoResponse for FinalizeTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("finalize template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── GET ───────────────────────────────────────────────────────────────────────

pub async fn finalize_team(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let mut team = match state.team_creation.roster_repository.find_by_id(&team_entity_id).await {
        Ok(Some(t)) => t,
        Ok(None)    => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("finalize_team repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let ref_data = state.team_creation.reference_data.as_ref();
    let routes   = Routes::default();

    let roster_def = ref_data.find_roster_definition(&team.roster.id.0);
    let roster_leagues: Vec<String> = roster_def
        .as_ref()
        .map(|d| d.leagues.clone())
        .unwrap_or_default();

    // ── Skip si pas de finalisation nécessaire ────────────────────────────────
    if !team.needs_finalization(roster_leagues.len()) {
        if team.league_id.is_none() {
            let league_id = LeagueId(roster_leagues[0].clone());
            if let Err(e) = crate::app::team_creation::use_cases::set_league::execute(
                SetLeagueCommand {
                    team_id:  team_entity_id.clone(),
                    space_id: space_id.clone(),
                    league_id,
                },
                state.team_creation.roster_repository.as_ref(),
            )
            .await
            {
                return match e {
                    SetLeagueError::TeamNotFound  => StatusCode::NOT_FOUND.into_response(),
                    SetLeagueError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                };
            }
        }

        let cmd = SubmitTeamCommand {
            team_id:    team_entity_id,
            space_id:   space_id.clone(),
            coach_name: user.coach_name.into_inner(),
        };
        return match submit_uc::execute(
            cmd,
            state.team_creation.roster_repository.as_ref(),
            &state.team_creation.event_bus,
        )
        .await
        {
            Ok(()) => Response::builder()
                .header("HX-Redirect", routes.my_teams(&space_id))
                .header("HX-Trigger", r#"{"showToast":"Équipe soumise avec succès !"}"#)
                .body(Body::empty())
                .unwrap()
                .into_response(),
            Err(submit_uc::SubmitTeamError::Domain(errors)) => {
                let msgs: String = errors
                    .iter()
                    .map(|e| format!("<p>{}</p>", submit_uc::domain_error_message(e)))
                    .collect();
                Response::builder()
                    .status(422)
                    .header("Content-Type", "text/html")
                    .body(Body::from(msgs))
                    .unwrap()
                    .into_response()
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // ── Construire les VMs ───────────────────────────────────────────────────
    team.assign_missing_jerseys();

    let players: Vec<FinalizePlayerVm> = team
        .hired_players()
        .iter()
        .map(|p| {
            let base_skills = ref_data.resolve_base_skills(&p.definition.id.0);
            let acquired_csv: String = p
                .acquired_skills
                .iter()
                .map(|a| a.skill_id.0.as_str())
                .collect::<Vec<_>>()
                .join(",");

            FinalizePlayerVm {
                id: p.instance_id.0.clone(),
                jersey: p.jersey.map(|j| j.0).unwrap_or(0),
                name: if p.personal_name.is_empty() {
                    p.definition.name.0.clone()
                } else {
                    p.personal_name.clone()
                },
                position_name: p.definition.name.0.clone(),
                roster_line_id: p.definition.id.0.clone(),
                base_skills,
                acquired_count: p.acquired_skills.len(),
                acquired_csv,
            }
        })
        .collect();

    let spp_log: Vec<SppLogEntryVm> = team
        .hired_players()
        .iter()
        .flat_map(|p| {
            p.acquired_skills.iter().map(move |a| {
                let skill_name = ref_data
                    .resolve_skill_name(&a.skill_id.0)
                    .unwrap_or_else(|| a.skill_id.0.clone());
                SppLogEntryVm {
                    player_id: p.instance_id.0.clone(),
                    player_name: if p.personal_name.is_empty() {
                        p.definition.name.0.clone()
                    } else {
                        p.personal_name.clone()
                    },
                    jersey: p.jersey.map(|j| j.0).unwrap_or(0),
                    position_name: p.definition.name.0.clone(),
                    skill_id: a.skill_id.0.clone(),
                    skill_name,
                    mode_label: match a.mode {
                        crate::app::team_creation::domain::roster::AcquisitionMode::Chosen => "Choisie".into(),
                        crate::app::team_creation::domain::roster::AcquisitionMode::Random => "Aléatoire".into(),
                    },
                    spp_cost: a.spp_cost,
                }
            })
        })
        .collect();

    let logo_url = team.base_infos().logo_url().map(|img| img.thumbnail(120, 120));

    FinalizeTeamTemplate {
        web_routes: WebRoutes::default(),
        team_routes: routes,
        ref_routes: RefRoutes::default(),
        space_id,
        team_id,
        logo_url,
        team_name: team.base_infos().name().clone().into_inner(),
        roster_name: team.roster.name.0.clone(),
        treasury: team.remaining_budget().unwrap_or(0),
        spp_pool: team.spp_pool,
        players,
        spp_log,
    }
    .into_response()
}

// ── POST ──────────────────────────────────────────────────────────────────────

pub async fn post_finalize_team(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = SubmitTeamCommand {
        team_id:    team_entity_id,
        space_id:   space_id.clone(),
        coach_name: user.coach_name.into_inner(),
    };

    let routes = Routes::default();

    match submit_uc::execute(
        cmd,
        state.team_creation.roster_repository.as_ref(),
        &state.team_creation.event_bus,
    )
    .await
    {
        Ok(()) => Response::builder()
            .header("HX-Redirect", routes.my_teams(&space_id))
            .header("HX-Trigger", r#"{"showToast":"Équipe soumise avec succès !"}"#)
            .body(Body::empty())
            .unwrap()
            .into_response(),
        Err(submit_uc::SubmitTeamError::TeamNotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(submit_uc::SubmitTeamError::Domain(ref errors)) => {
            let msgs: String = errors
                .iter()
                .map(|e| format!(
                    r#"<p class="table-error">{}</p>"#,
                    submit_uc::domain_error_message(e)
                ))
                .collect();
            Response::builder()
                .header("HX-Retarget", "#submit-errors")
                .header("HX-Reswap", "innerHTML")
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(msgs))
                .unwrap()
                .into_response()
        }
        Err(submit_uc::SubmitTeamError::Repository(e)) => {
            tracing::error!("post_finalize_team repo error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
