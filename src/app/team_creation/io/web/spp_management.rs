use crate::app::auth::auth_backend::AuthSession;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::team_creation::domain::roster::{AcquisitionMode, PlayerId, SkillId};
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::routes::Routes;
use crate::app::team_creation::use_cases::cancel_creation_spp as cancel_uc;
use crate::app::team_creation::use_cases::cancel_creation_spp::CancelCreationSppCommand;
use crate::app::team_creation::use_cases::spend_creation_spp as spend_uc;
use crate::app::team_creation::use_cases::spend_creation_spp::SpendCreationSppCommand;
use crate::app::shared_kernel::common_types::EntityId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct SpendSppBody {
    pub skill_id: String,
    pub mode:     String,
}

// ── View models ───────────────────────────────────────────────────────────────

pub struct SppSummaryEntryVm {
    pub instance_id:    String,
    pub player_name:    String,
    pub position_name:  String,
    pub jersey:         u8,
    pub skill_name:     String,
    pub category_label: String,
    pub category_css:   String,
    pub mode_label:     String,
    pub spp_cost:       u8,
    pub cancel_url:     String,
}

pub struct SppSummaryVm {
    pub entries:    Vec<SppSummaryEntryVm>,
    pub spp_spent:  u8,
    pub spp_remaining: u8,
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "finalize-spp-summary-fragment.html")]
pub struct SppSummaryTemplate {
    pub summary: SppSummaryVm,
}

impl IntoResponse for SppSummaryTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub fn build_summary(
    team: &RosterSelectedTeam,
    ref_repo: &dyn IReferenceRepository,
    routes: &Routes,
    space_id: &str,
    team_id: &str,
    _initial_pool: u8,
) -> SppSummaryVm {
    let mut entries = Vec::new();
    let mut spp_spent: u8 = 0;

    for player in team.hired_players() {
        for acq in &player.acquired_skills {
            let skill_name = ref_repo
                .find_skill_by_uid(&acq.skill_id.0)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| acq.skill_id.0.clone());
            let (cat_label, cat_css) = ref_repo
                .find_skill_by_uid(&acq.skill_id.0)
                .map(|s| {
                    let label = ref_repo
                        .list_skill_categories()
                        .iter()
                        .find(|c| c.id == s.category)
                        .map(|c| c.label.clone())
                        .unwrap_or_else(|| s.category.clone());
                    let css = match s.category.as_str() {
                        "GENERAL"  => "type-general",
                        "STRENGTH" => "type-strength",
                        "AGILITY"  => "type-agility",
                        "PASSING"  => "type-passing",
                        "MUTATION" => "type-mutation",
                        _          => "type-general",
                    };
                    (label, css.to_string())
                })
                .unwrap_or_else(|| (String::new(), "type-general".into()));

            spp_spent = spp_spent.saturating_add(acq.spp_cost);
            entries.push(SppSummaryEntryVm {
                cancel_url: routes.cancel_spp(
                    space_id,
                    team_id,
                    &player.instance_id.0,
                    &acq.skill_id.0,
                ),
                instance_id: player.instance_id.0.clone(),
                player_name: if player.personal_name.is_empty() {
                    player.definition.name.0.clone()
                } else {
                    player.personal_name.clone()
                },
                position_name: player.definition.name.0.clone(),
                jersey: player.jersey.unwrap_or(0),
                skill_name,
                category_label: cat_label,
                category_css: cat_css,
                mode_label: match acq.mode {
                    AcquisitionMode::Chosen => "Choisie".into(),
                    AcquisitionMode::Random => "Aléatoire".into(),
                },
                spp_cost: acq.spp_cost,
            });
        }
    }

    SppSummaryVm {
        entries,
        spp_spent,
        spp_remaining: team.spp_pool,
    }
}

fn spp_trigger(team: &RosterSelectedTeam, instance_id: &str, routes: &Routes, space_id: &str, team_id: &str) -> String {
    let player = team.hired_players().iter().find(|p| p.instance_id.0 == instance_id);
    let (roster_line_id, acquired, on_acquire, on_cancel) = player
        .map(|p| (
            p.definition.id.0.clone(),
            p.acquired_skill_ids_csv(),
            routes.spend_spp(space_id, team_id, &p.instance_id.0),
            routes.cancel_spp_base(space_id, team_id, &p.instance_id.0),
        ))
        .unwrap_or_default();

    json!({
        "skillsUpdated": {
            "roster_line_id": roster_line_id,
            "spp":            team.spp_pool,
            "acquired":       acquired,
            "on_acquire":     on_acquire,
            "on_cancel":      on_cancel
        }
    })
    .to_string()
}

// ── Handler spend ─────────────────────────────────────────────────────────────

pub async fn spend_spp(
    _auth_session: AuthSession,
    Path((space_id, team_id, instance_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<SpendSppBody>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let mode = if body.mode == "random" {
        AcquisitionMode::Random
    } else {
        AcquisitionMode::Chosen
    };
    let is_elite = state
        .references
        .repository
        .find_skill_by_uid(&body.skill_id)
        .map(|s| s.skill_type == "Élite")
        .unwrap_or(false);
    let spp_cost = match (mode, is_elite) {
        (AcquisitionMode::Chosen, false) => 3,
        (AcquisitionMode::Chosen, true)  => 6,
        (AcquisitionMode::Random, false) => 2,
        (AcquisitionMode::Random, true)  => 4,
    };

    let cmd = SpendCreationSppCommand {
        team_id: team_entity_id.clone(),
        space_id: space_id.clone(),
        instance_id: PlayerId(instance_id.clone()),
        skill_id: SkillId(body.skill_id.clone()),
        mode,
        spp_cost,
    };

    if let Err(e) = spend_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        return match e {
            spend_uc::SpendCreationSppError::TeamNotFound => StatusCode::NOT_FOUND.into_response(),
            spend_uc::SpendCreationSppError::Domain(d) => axum::http::Response::builder()
                .status(422)
                .header("Content-Type", "text/html")
                .body(axum::body::Body::from(format!(r#"<div class="spp-error">{d}</div>"#)))
                .unwrap()
                .into_response(),
            spend_uc::SpendCreationSppError::Repository(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }

    let team = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_entity_id)
        .await
    {
        Ok(Some(t)) => t,
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let routes = Routes::default();
    let ref_repo = state.references.repository.as_ref();
    let initial_pool = team.spp_pool + spp_cost; // approximate initial = remaining + spent
    let summary = build_summary(&team, ref_repo, &routes, &space_id, &team_id, initial_pool);
    let trigger = spp_trigger(&team, &instance_id, &routes, &space_id, &team_id);

    let summary_html = SppSummaryTemplate { summary }.render().unwrap_or_default();

    axum::http::Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .header("HX-Trigger", trigger)
        .body(axum::body::Body::from(summary_html))
        .unwrap()
        .into_response()
}

// ── Handler cancel ────────────────────────────────────────────────────────────

pub async fn cancel_spp(
    _auth_session: AuthSession,
    Path((space_id, team_id, instance_id, skill_id)): Path<(String, String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = CancelCreationSppCommand {
        team_id: team_entity_id.clone(),
        space_id: space_id.clone(),
        instance_id: PlayerId(instance_id.clone()),
        skill_id: SkillId(skill_id),
    };

    if let Err(e) = cancel_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        return match e {
            cancel_uc::CancelCreationSppError::TeamNotFound => StatusCode::NOT_FOUND.into_response(),
            cancel_uc::CancelCreationSppError::Domain(_) | cancel_uc::CancelCreationSppError::Repository(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }

    let team = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_entity_id)
        .await
    {
        Ok(Some(t)) => t,
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let routes = Routes::default();
    let ref_repo = state.references.repository.as_ref();
    let summary = build_summary(&team, ref_repo, &routes, &space_id, &team_id, 0);
    let trigger = spp_trigger(&team, &instance_id, &routes, &space_id, &team_id);

    let summary_html = SppSummaryTemplate { summary }.render().unwrap_or_default();

    axum::http::Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .header("HX-Trigger", trigger)
        .body(axum::body::Body::from(summary_html))
        .unwrap()
        .into_response()
}
