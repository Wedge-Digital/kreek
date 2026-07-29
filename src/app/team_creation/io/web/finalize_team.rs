use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::io::web::view_models::SppLogEntryVm;
use crate::app::team_creation::use_cases::commands::SubmitTeamCommand;
use crate::app::team_creation::use_cases::roster_service;
use crate::app::team_creation::use_cases::submit_team as submit_uc;
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
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
    pub logo_url: Option<String>,
    pub team_name: String,
    pub roster_name: String,
    pub treasury: u32,
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

/// Le lien "Terminer la construction" est un `hx-get` de navigation de page,
/// avec `hx-select="#app-content"` pour n'extraire que ce fragment de la page
/// finalize-team en cas de succès. `HX-Retarget` seul ne suffit donc pas pour
/// une erreur : `hx-select` filtrerait la réponse et n'y trouverait aucun
/// `#app-content`, donc rien ne s'afficherait. `HX-Reselect` remplace le
/// sélecteur pour cette réponse précise, en le faisant correspondre au conteneur
/// `#submit-errors` déjà présent sur build-team.html (et donc au retarget).
fn submit_error_response(message: String) -> Response {
    Response::builder()
        .header("HX-Retarget", "#submit-errors")
        .header("HX-Reselect", "#submit-errors")
        .header("HX-Reswap", "outerHTML")
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(format!(
            r#"<div id="submit-errors" class="table-error-zone"><p class="table-error">{message}</p></div>"#
        )))
        .unwrap()
        .into_response()
}

// ── GET ───────────────────────────────────────────────────────────────────────

pub async fn finalize_team(
    auth_session: AuthSession,
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(_user) = auth_session.user else {
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
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("finalize_team repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let draft = match state
        .team_creation
        .team_repository
        .find_by_id(&team_entity_id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("finalize_team draft find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let auto_enroll = match SeasonId::try_new(draft.season_id()) {
        Ok(sid) => state
            .competitions
            .season_repository
            .find_invitations(&sid)
            .await
            .ok()
            .flatten()
            .map(|inv| !inv.requires_validation.0)
            .unwrap_or(false),
        Err(_) => false,
    };

    let draft_competition_name = state
        .team_creation
        .competition_display
        .find_competition_name(draft.competition_id())
        .await
        .unwrap_or_default();
    let draft_season_name = state
        .team_creation
        .competition_display
        .find_season_name(draft.season_id())
        .await
        .unwrap_or_default();

    let ref_data = state.team_creation.reference_data.as_ref();

    // ── Validation des prérequis ─────────────────────────────────────────────
    use crate::app::team_creation::domain::team_roster_selected::MIN_PLAYERS_FOR_SUBMISSION;
    if team.hired_players().len() < MIN_PLAYERS_FOR_SUBMISSION {
        return submit_error_response(format!(
            "Vous devez recruter au moins {} joueurs avant de finaliser.",
            MIN_PLAYERS_FOR_SUBMISSION
        ));
    }

    let league_selection_missing = roster_service::league_selection_missing(
        &team.roster.id.0,
        team.league_id.is_some(),
        ref_data,
    );

    if league_selection_missing {
        return submit_error_response(
            "Veuillez sélectionner une ligue avant de terminer la construction.".to_string(),
        );
    }

    let special_rule_selection_missing = roster_service::special_rule_selection_missing(
        &team.roster.id.0,
        team.special_rule_id.is_some(),
        ref_data,
    );

    if special_rule_selection_missing {
        return submit_error_response(
            "Veuillez sélectionner une règle spéciale avant de terminer la construction."
                .to_string(),
        );
    }

    // ── Skip si pas de finalisation nécessaire ────────────────────────────────
    if !team.needs_finalization() {
        let cmd = SubmitTeamCommand {
            team_id: team_entity_id,
            space_id: space_id.clone(),
            competition_id: draft.competition_id().to_string(),
            competition_name: draft_competition_name.clone(),
            season_id: draft.season_id().to_string(),
            season_name: draft_season_name.clone(),
            coach_name: draft.coach_name().to_string(),
            auto_enroll,
        };
        return match submit_uc::execute(
            cmd,
            state.team_creation.roster_repository.as_ref(),
            &state.team_creation.event_bus,
        )
        .await
        {
            Ok(()) => Response::builder()
                .header(
                    "HX-Redirect",
                    AppRoutes::default().teams.team_detail(&space_id, &team_id),
                )
                .header(
                    "HX-Trigger",
                    r#"{"showToast":"Équipe soumise avec succès !"}"#,
                )
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
                        p.definition.name.as_ref().to_string()
                    } else {
                        p.personal_name.clone()
                    },
                    jersey: p.jersey.map(|j| j.into_inner()).unwrap_or(0),
                    position_name: p.definition.name.as_ref().to_string(),
                    skill_id: a.skill_id.0.clone(),
                    skill_name,
                    mode_label: match a.mode {
                        crate::app::team_creation::domain::roster::AcquisitionMode::Chosen => {
                            "Choisie".into()
                        }
                        crate::app::team_creation::domain::roster::AcquisitionMode::Random => {
                            "Aléatoire".into()
                        }
                    },
                    spp_cost: a.spp_cost.into_inner(),
                }
            })
        })
        .collect();

    let logo_url = team
        .base_infos()
        .logo_url()
        .map(|img| img.thumbnail(120, 120));

    FinalizeTeamTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        team_id,
        logo_url,
        team_name: team.base_infos().name().clone().into_inner(),
        roster_name: team.roster.name.as_ref().to_string(),
        treasury: team.remaining_budget().unwrap_or(0),
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
    let Some(_user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let post_draft = match state
        .team_creation
        .team_repository
        .find_by_id(&team_entity_id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let post_auto_enroll = match SeasonId::try_new(post_draft.season_id()) {
        Ok(sid) => state
            .competitions
            .season_repository
            .find_invitations(&sid)
            .await
            .ok()
            .flatten()
            .map(|inv| !inv.requires_validation.0)
            .unwrap_or(false),
        Err(_) => false,
    };

    let post_comp_name = state
        .team_creation
        .competition_display
        .find_competition_name(post_draft.competition_id())
        .await
        .unwrap_or_default();
    let post_season_name = state
        .team_creation
        .competition_display
        .find_season_name(post_draft.season_id())
        .await
        .unwrap_or_default();

    let cmd = SubmitTeamCommand {
        team_id: team_entity_id,
        space_id: space_id.clone(),
        competition_id: post_draft.competition_id().to_string(),
        competition_name: post_comp_name,
        season_id: post_draft.season_id().to_string(),
        season_name: post_season_name,
        coach_name: post_draft.coach_name().to_string(),
        auto_enroll: post_auto_enroll,
    };

    match submit_uc::execute(
        cmd,
        state.team_creation.roster_repository.as_ref(),
        &state.team_creation.event_bus,
    )
    .await
    {
        Ok(()) => Response::builder()
            .header(
                "HX-Redirect",
                AppRoutes::default().teams.team_detail(&space_id, &team_id),
            )
            .header(
                "HX-Trigger",
                r#"{"showToast":"Équipe soumise avec succès !"}"#,
            )
            .body(Body::empty())
            .unwrap()
            .into_response(),
        Err(submit_uc::SubmitTeamError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(submit_uc::SubmitTeamError::Domain(ref errors)) => {
            let msgs: String = errors
                .iter()
                .map(|e| {
                    format!(
                        r#"<p class="table-error">{}</p>"#,
                        submit_uc::domain_error_message(e)
                    )
                })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Régression : le lien "Terminer la construction" est un hx-get avec
    /// hx-select="#app-content" (navigation de page). HX-Retarget seul est
    /// filtré par ce hx-select côté client et n'affiche rien — HX-Reselect
    /// doit pointer vers un sélecteur présent dans le corps de la réponse.
    #[test]
    fn submit_error_response_overrides_client_side_select_to_avoid_empty_swap() {
        let response = submit_error_response("Message de test".to_string());
        let headers = response.headers();

        assert_eq!(headers.get("HX-Retarget").unwrap(), "#submit-errors");
        assert_eq!(headers.get("HX-Reselect").unwrap(), "#submit-errors");
        assert_eq!(headers.get("HX-Reswap").unwrap(), "outerHTML");
    }
}
