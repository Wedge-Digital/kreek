use crate::app::auth::auth_backend::AuthSession;
use crate::app::references::domain::models::ChosenSkillCost;
use crate::app::references::routes::Routes as RefRoutes;
use crate::app::team_creation::domain::roster::{AcquisitionMode, LeagueId, PlayerId, SkillId};
use crate::app::team_creation::routes::Routes;
use crate::app::team_creation::use_cases::batch_finalize::{
    self as batch_uc, BatchFinalizeCommand, SkillAssignment,
};
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
use serde::{Deserialize, Serialize};

// ── POST body ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AssignmentRequest {
    pub player_id: String,
    pub skill_id:  String,
    pub mode:      String,
}

// ── Serializable page data (FINALIZE_DATA) ────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerJson {
    id:                 String,
    jersey:             u8,
    name:               String,
    position_name:      String,
    roster_line_id:     String,
    base_skills:        Vec<String>,
    existing_skill_ids: Vec<String>,
}

#[derive(Serialize)]
struct PricingJson {
    chosen: ChosenSkillCost,
    random: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FinalizeData {
    team_name:             String,
    roster_name:           String,
    treasury:              u32,
    spp_pool:              u8,
    pricing:               PricingJson,
    submit_url:            String,
    skill_picker_base_url: String,
    players:               Vec<PlayerJson>,
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "finalize-team.html")]
pub struct FinalizeTeamTemplate {
    pub web_routes:          WebRoutes,
    pub team_routes:         Routes,
    pub space_id:            String,
    pub team_id:             String,
    pub league_selector_url: String,
    pub set_league_url:      String,
    pub data_json:           String,
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

    let team = match state.team_creation.roster_repository.find_by_id(&team_entity_id).await {
        Ok(Some(t)) => t,
        Ok(None)    => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("finalize_team repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let ref_repo = state.references.repository.as_ref();
    let routes   = Routes::default();

    let roster_leagues: Vec<String> = ref_repo
        .list_teams()
        .iter()
        .find(|t| t.uid == team.roster.id.0)
        .map(|t| t.leagues.clone())
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

    // ── Construire FINALIZE_DATA ──────────────────────────────────────────────
    let pricing_level = ref_repo
        .skill_cost_matrix()
        .iter()
        .find(|l| l.level == 1)
        .expect("level 1 must exist in skill cost matrix");

    let mut used_jerseys: std::collections::HashSet<u8> = team
        .hired_players()
        .iter()
        .filter_map(|p| p.jersey)
        .collect();
    let mut next_jersey = 1u8;

    let players: Vec<PlayerJson> = team
        .hired_players()
        .iter()
        .map(|p| {
            let jersey = match p.jersey {
                Some(j) => j,
                None => {
                    while used_jerseys.contains(&next_jersey) { next_jersey += 1; }
                    let j = next_jersey;
                    used_jerseys.insert(j);
                    next_jersey += 1;
                    j
                }
            };

            let base_skills: Vec<String> = ref_repo
                .find_position_by_uid(&p.definition.id.0)
                .map(|pos| {
                    pos.skills
                        .iter()
                        .filter_map(|uid| ref_repo.find_skill_by_uid(uid))
                        .map(|s| s.name.clone())
                        .collect()
                })
                .unwrap_or_default();

            let existing_skill_ids: Vec<String> = p
                .acquired_skills
                .iter()
                .map(|a| a.skill_id.0.clone())
                .collect();

            PlayerJson {
                id:                 p.instance_id.0.clone(),
                jersey,
                name:               if p.personal_name.is_empty() {
                    p.definition.name.0.clone()
                } else {
                    p.personal_name.clone()
                },
                position_name:      p.definition.name.0.clone(),
                roster_line_id:     p.definition.id.0.clone(),
                base_skills,
                existing_skill_ids,
            }
        })
        .collect();

    let data = FinalizeData {
        team_name:    team.base_infos().name().clone().into_inner(),
        roster_name:  team.roster.name.0.clone(),
        treasury:     team.remaining_budget().unwrap_or(0),
        spp_pool:     team.spp_pool,
        pricing: PricingJson {
            chosen: pricing_level.chosen.clone(),
            random: pricing_level.random,
        },
        submit_url:            routes.finalize_team(&space_id, &team_id),
        skill_picker_base_url: RefRoutes::default().skill_picker_base().to_string(),
        players,
    };

    let data_json = serde_json::to_string(&data).expect("FINALIZE_DATA serialization");

    let ref_routes       = RefRoutes::default();
    let set_league_url   = routes.set_league(&space_id, &team_id);
    let league_selector_url = ref_routes.league_selector(
        team.league_id.as_ref().map(|l| l.0.as_str()).unwrap_or(""),
        &set_league_url,
    );

    FinalizeTeamTemplate {
        web_routes: WebRoutes::default(),
        team_routes: routes,
        space_id: space_id.clone(),
        team_id: team_id.clone(),
        league_selector_url,
        set_league_url,
        data_json,
    }
    .into_response()
}

// ── POST ──────────────────────────────────────────────────────────────────────

pub async fn post_finalize_team(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<Vec<AssignmentRequest>>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let team = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_entity_id)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None)    => return StatusCode::NOT_FOUND.into_response(),
        Err(_)      => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let ref_repo      = state.references.repository.as_ref();
    let pricing_level = ref_repo
        .skill_cost_matrix()
        .iter()
        .find(|l| l.level == 1)
        .expect("level 1 must exist");

    let mut assignments = Vec::new();
    for req in &body {
        let mode = if req.mode == "random" {
            AcquisitionMode::Random
        } else {
            AcquisitionMode::Chosen
        };

        let Some(player) = team.hired_players().iter().find(|p| p.instance_id.0 == req.player_id) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let Some(skill) = ref_repo.find_skill_by_uid(&req.skill_id) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let Some(position) = ref_repo.find_position_by_uid(&player.definition.id.0) else {
            return StatusCode::BAD_REQUEST.into_response();
        };

        let is_primary = position.primary_access.contains(&skill.category);
        let spp_cost = match (mode, is_primary) {
            (AcquisitionMode::Chosen, true)  => pricing_level.chosen.primary,
            (AcquisitionMode::Chosen, false) => pricing_level.chosen.secondary,
            (AcquisitionMode::Random, _)     => pricing_level.random,
        };

        assignments.push(SkillAssignment {
            player_id: PlayerId(req.player_id.clone()),
            skill_id:  SkillId(req.skill_id.clone()),
            mode,
            spp_cost,
        });
    }

    let cmd = BatchFinalizeCommand {
        team_id:     team_entity_id,
        space_id:    space_id.clone(),
        coach_name:  user.coach_name.into_inner(),
        assignments,
    };

    let routes = Routes::default();

    match batch_uc::execute(
        cmd,
        state.team_creation.roster_repository.as_ref(),
        &state.team_creation.event_bus,
    )
    .await
    {
        Ok(()) => Response::builder()
            .header("HX-Redirect", routes.my_teams(&space_id))
            .body(Body::empty())
            .unwrap()
            .into_response(),
        Err(batch_uc::BatchFinalizeError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(batch_uc::BatchFinalizeError::Domain(errors)) => {
            let msgs: String = errors
                .iter()
                .map(|e| format!("<p>{}</p>", batch_uc::domain_error_message(e)))
                .collect();
            Response::builder()
                .status(422)
                .header("Content-Type", "text/html")
                .body(Body::from(msgs))
                .unwrap()
                .into_response()
        }
        Err(batch_uc::BatchFinalizeError::Repository(_)) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
