use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::routes::Routes;
use crate::app::competitions::use_cases::create_draft_competition::{
    execute, CreateDraftCompetitionCommand, CreateDraftCompetitionError,
};
use crate::app::competitions::use_cases::save_competition_rules::{
    execute as execute_save_rules, SaveCompetitionRulesCommand, SaveCompetitionRulesError,
};
use crate::app::competitions::use_cases::update_draft_competition::{
    execute as execute_update, UpdateDraftCompetitionCommand, UpdateDraftCompetitionError,
};
use crate::app::references::io::web::pickers::{
    build_roster_items, build_star_player_items, InducementPickerItem,
    RosterPickerItem, StarPlayerPickerItem,
};
use crate::app::shared_kernel::common_types::{
    CloudinaryImage, CoachId, CompetitionId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::competition_name::CompetitionName;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

// ── Phase 2 ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "new-competition-phase-2.html")]
pub struct NewCompetitionPhase2Template {
    pub web_routes: WebRoutes,
    pub competition_routes: Routes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub season_name_value: String,
    pub rosters: Vec<RosterPickerItem>,
    pub star_players: Vec<StarPlayerPickerItem>,
    pub existing_rules_json: String,
}

impl IntoResponse for NewCompetitionPhase2Template {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_2(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let season_repo = state.competitions.season_repository.as_ref();

    let (rules_result, base_result) = tokio::join!(
        season_repo.find_rules(&sid),
        season_repo.find_base_info(&sid),
    );

    let existing_rules_json = rules_result
        .ok()
        .flatten()
        .and_then(|r| serde_json::to_string(&r).ok())
        .unwrap_or_else(|| "null".to_string());

    let season_name_value = base_result
        .ok()
        .flatten()
        .map(|b| b.name)
        .unwrap_or_else(|| "Saison 1".to_string());

    let refs = state.references.repository.as_ref();
    NewCompetitionPhase2Template {
        web_routes: WebRoutes,
        competition_routes: Routes,
        space_id,
        competition_id,
        season_id,
        season_name_value,
        rosters: build_roster_items(refs),
        star_players: build_star_player_items(refs),
        existing_rules_json,
    }
    .into_response()
}

// ── Phase 1 ──────────────────────────────────────────────────────────────────

#[derive(Template, Default)]
#[template(path = "new-competition-phase-1.html")]
pub struct NewCompetitionTemplate {
    pub web_routes: WebRoutes,
    pub space_id: String,
    pub competition_id: Option<String>,
    pub season_id: Option<String>,
    pub members_widget_url: String,
    pub name_value: String,
    pub name_error: Option<String>,
    pub logo_url_value: String,
    pub logo_error: Option<String>,
    pub general_error: Option<String>,
    pub competition_routes: Routes,
}

impl IntoResponse for NewCompetitionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn hx_redirect(url: impl Into<String>) -> Response {
    match Response::builder()
        .header("HX-Redirect", url.into())
        .body(Body::empty())
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("hx_redirect build failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn members_widget_url(space_id: &str, selected_ids: &[String]) -> String {
    if selected_ids.is_empty() {
        format!("/app/{space_id}/members-widget")
    } else {
        format!(
            "/app/{space_id}/members-widget?selected={}",
            selected_ids.join(",")
        )
    }
}

pub async fn get_new_competition_phase_1(Path(space_id): Path<String>) -> impl IntoResponse {
    let url = members_widget_url(&space_id, &[]);
    NewCompetitionTemplate {
        space_id,
        members_widget_url: url,
        ..Default::default()
    }
    .into_response()
}

pub async fn get_new_competition_phase_1_edit(
    Path((space_id, competition_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let base = match state
        .competitions
        .competition_repository
        .find_base_info(&cid)
        .await
    {
        Ok(Some(b)) => b,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let season_id = state
        .competitions
        .season_repository
        .find_latest_season_id(&cid)
        .await
        .ok()
        .flatten()
        .map(|s| s.to_string());

    let url = members_widget_url(&space_id, &base.admin_ids);
    NewCompetitionTemplate {
        members_widget_url: url,
        competition_id: Some(competition_id),
        season_id,
        name_value: base.name,
        logo_url_value: base.logo.unwrap_or_default(),
        space_id,
        ..Default::default()
    }
    .into_response()
}

// ── POST: création ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateCompetitionFormPayload {
    pub name: String,
    pub logo_url: String,
    #[serde(default)]
    pub admin_ids: Vec<String>,
    pub season_id: Option<String>,
}

pub async fn post_new_competition(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<CreateCompetitionFormPayload>,
) -> impl IntoResponse {
    let url = members_widget_url(&space_id, &[]);
    let mut tmpl = NewCompetitionTemplate {
        space_id: space_id.clone(),
        members_widget_url: url,
        name_value: payload.name.clone(),
        logo_url_value: payload.logo_url.clone(),
        ..Default::default()
    };

    let sid = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => {
            tmpl.general_error = Some("Espace invalide.".into());
            return tmpl.into_response();
        }
    };

    let name = match CompetitionName::try_new(&payload.name) {
        Ok(v) => Some(v),
        Err(_) => {
            tmpl.name_error = Some(
                "Le nom peut contenir lettres, chiffres, espaces et ponctuation courante (100 caractères max).".into(),
            );
            None
        }
    };

    let logo = match CloudinaryImage::try_new(payload.logo_url.clone()) {
        Ok(v) => Some(v),
        Err(_) => {
            tmpl.logo_error = Some("Veuillez uploader un logo pour votre compétition.".into());
            None
        }
    };

    let (Some(name), Some(logo)) = (name, logo) else {
        return tmpl.into_response();
    };

    let Some(user) = auth_session.user else {
        return hx_redirect(crate::app::auth::routes::path::AUTH_LAYOUT);
    };

    let admin_ids: Vec<CoachId> = payload
        .admin_ids
        .iter()
        .filter_map(|id| CoachId::try_new(id).ok())
        .collect();

    let cmd = CreateDraftCompetitionCommand {
        space_id: sid,
        created_by: user.id,
        name,
        logo,
        admin_ids,
    };

    match execute(
        cmd,
        state.competitions.competition_repository.as_ref(),
        state.competitions.season_repository.as_ref(),
        &state.competitions.event_bus,
    )
    .await
    {
        Ok((competition_id, season_id)) => hx_redirect(Routes.new_competition_rules(
            &space_id,
            &competition_id.to_string(),
            &season_id.to_string(),
        )),

        Err(CreateDraftCompetitionError::CompetitionNameAlreadyTaken) => {
            tmpl.name_error =
                Some("Une compétition avec ce nom existe déjà dans cet espace.".into());
            tmpl.into_response()
        }

        Err(CreateDraftCompetitionError::Database(_)) => {
            tmpl.general_error = Some("Erreur interne, veuillez réessayer.".into());
            tmpl.into_response()
        }
    }
}

// ── POST: mise à jour ─────────────────────────────────────────────────────────

pub async fn post_update_competition(
    auth_session: AuthSession,
    Path((space_id, competition_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<CreateCompetitionFormPayload>,
) -> impl IntoResponse {
    let url = members_widget_url(&space_id, &payload.admin_ids);
    let mut tmpl = NewCompetitionTemplate {
        space_id: space_id.clone(),
        competition_id: Some(competition_id.clone()),
        season_id: payload.season_id.clone(),
        members_widget_url: url,
        name_value: payload.name.clone(),
        logo_url_value: payload.logo_url.clone(),
        ..Default::default()
    };

    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => {
            tmpl.general_error = Some("Identifiant de compétition invalide.".into());
            return tmpl.into_response();
        }
    };

    let sid = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => {
            tmpl.general_error = Some("Espace invalide.".into());
            return tmpl.into_response();
        }
    };

    let season_id = match payload
        .season_id
        .as_deref()
        .and_then(|s| SeasonId::try_new(s).ok())
    {
        Some(id) => id,
        None => {
            tmpl.general_error = Some("Identifiant de saison manquant.".into());
            return tmpl.into_response();
        }
    };

    let name = match CompetitionName::try_new(&payload.name) {
        Ok(v) => Some(v),
        Err(_) => {
            tmpl.name_error = Some(
                "Le nom peut contenir lettres, chiffres, espaces et ponctuation courante (100 caractères max).".into(),
            );
            None
        }
    };

    let logo = match CloudinaryImage::try_new(payload.logo_url.clone()) {
        Ok(v) => Some(v),
        Err(_) => {
            tmpl.logo_error = Some("Veuillez uploader un logo pour votre compétition.".into());
            None
        }
    };

    let (Some(name), Some(logo)) = (name, logo) else {
        return tmpl.into_response();
    };

    if auth_session.user.is_none() {
        return hx_redirect(crate::app::auth::routes::path::AUTH_LAYOUT);
    }

    let admin_ids: Vec<CoachId> = payload
        .admin_ids
        .iter()
        .filter_map(|id| CoachId::try_new(id).ok())
        .collect();

    let cmd = UpdateDraftCompetitionCommand {
        competition_id: cid,
        space_id: sid,
        name,
        logo,
        admin_ids,
    };

    match execute_update(cmd, state.competitions.competition_repository.as_ref()).await {
        Ok(()) => hx_redirect(Routes.new_competition_rules(
            &space_id,
            &competition_id,
            &season_id.to_string(),
        )),

        Err(UpdateDraftCompetitionError::CompetitionNameAlreadyTaken) => {
            tmpl.name_error =
                Some("Une compétition avec ce nom existe déjà dans cet espace.".into());
            tmpl.into_response()
        }

        Err(UpdateDraftCompetitionError::CompetitionNotFound) => {
            tmpl.general_error = Some("Compétition introuvable.".into());
            tmpl.into_response()
        }

        Err(UpdateDraftCompetitionError::Database(_)) => {
            tmpl.general_error = Some("Erreur interne, veuillez réessayer.".into());
            tmpl.into_response()
        }
    }
}

// ── POST phase 2 : sauvegarde des règles ─────────────────────────────────────

#[derive(Deserialize)]
pub struct SaveRulesPayload {
    pub season_name: String,
    #[serde(flatten)]
    pub rules: CompetitionRules,
}

pub async fn post_competition_rules(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<SaveRulesPayload>,
) -> impl IntoResponse {
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Identifiant de saison invalide.").into_response()
        }
    };

    let cmd = SaveCompetitionRulesCommand {
        season_id: sid,
        season_name: payload.season_name,
        rules: payload.rules,
    };

    match execute_save_rules(cmd, state.competitions.season_repository.as_ref()).await {
        Ok(()) => {
            hx_redirect(Routes.new_competition_structure(&space_id, &competition_id, &season_id))
        }

        Err(SaveCompetitionRulesError::RosterInMultipleTiers {
            roster,
            tiers: (t1, t2),
        }) => {
            let msg = format!(
                "Le roster « {} » est présent dans « {} » et « {} ».",
                roster, t1, t2
            );
            (StatusCode::UNPROCESSABLE_ENTITY, msg).into_response()
        }

        Err(SaveCompetitionRulesError::SeasonNotFound) => {
            (StatusCode::NOT_FOUND, "Saison introuvable.").into_response()
        }

        Err(SaveCompetitionRulesError::Database(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Erreur interne, veuillez réessayer.",
        )
            .into_response(),
    }
}
