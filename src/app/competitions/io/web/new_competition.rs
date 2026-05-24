use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::routes::Routes;
use crate::app::competitions::use_cases::create_draft_competition::{CreateDraftCompetitionCommand, CreateDraftCompetitionError, execute};
use crate::app::competitions::use_cases::save_competition_rules::{SaveCompetitionRulesCommand, SaveCompetitionRulesError, execute as execute_save_rules};
use crate::app::references::io::web::pickers::{
    build_inducement_items, build_roster_items, build_star_player_items,
    InducementPickerItem, RosterPickerItem, StarPlayerPickerItem,
};
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::competition_name::CompetitionName;
use crate::state::AppState;
use crate::web::app_layout::AppLayout;

#[derive(Template)]
#[template(path = "new-competition-phase-2.html")]
pub struct NewCompetitionPhase2Template {
    pub competition_routes:    Routes,
    pub space_id:              String,
    pub competition_id:        String,
    pub rosters:               Vec<RosterPickerItem>,
    pub inducements:           Vec<InducementPickerItem>,
    pub star_players:          Vec<StarPlayerPickerItem>,
    pub existing_rules_json:   String,
}

impl IntoResponse for NewCompetitionPhase2Template {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_2(
    Path((space_id, competition_id)): Path<(String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let cid = match crate::app::shared_kernel::common_types::CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let existing_rules_json = state.competitions.competition_repository
        .find_rules(&cid)
        .await
        .ok()
        .flatten()
        .and_then(|r| serde_json::to_string(&r).ok())
        .unwrap_or_else(|| "null".to_string());

    let refs = state.references.repository.as_ref();
    let tmpl = NewCompetitionPhase2Template {
        competition_routes: Routes,
        space_id,
        competition_id,
        rosters:             build_roster_items(refs),
        inducements:         build_inducement_items(refs),
        star_players:        build_star_player_items(refs),
        existing_rules_json,
    };
    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}

#[derive(Template, Default)]
#[template(path = "new-competition-phase-1.html")]
pub struct NewCompetitionTemplate {
    pub space_id:          String,
    pub name_value:        String,
    pub name_error:        Option<String>,
    pub logo_url_value:    String,
    pub logo_error:        Option<String>,
    pub general_error:     Option<String>,
    pub competition_routes: Routes,
}

impl IntoResponse for NewCompetitionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_1(
    Path(space_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let tmpl = NewCompetitionTemplate { space_id, ..Default::default() };
    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}

// ── Fragment: liste des membres pour le widget admins ────────────────────────

pub struct MemberItem {
    pub id:   String,
    pub name: String,
}

#[derive(Template)]
#[template(path = "competition-members-widget.html")]
pub struct MembersWidgetTemplate {
    pub members: Vec<MemberItem>,
}

impl IntoResponse for MembersWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_members_widget(
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sid = match SpaceId::try_new(&space_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cached = state
        .competitions
        .competitions_cache_repository
        .list_members_for_space(&sid)
        .await
        .unwrap_or_default();

    let members = cached
        .into_iter()
        .map(|u| MemberItem {
            id:   u.id.to_string(),
            name: u.coach_name.into_inner(),
        })
        .collect();

    MembersWidgetTemplate { members }.into_response()
}

// ── POST: soumettre la création du brouillon ─────────────────────────────────

#[derive(Deserialize)]
pub struct CreateCompetitionFormPayload {
    pub name:      String,
    pub logo_url:  String,
    #[serde(default)]
    pub admin_ids: Vec<String>,
}

pub async fn post_new_competition(
    auth_session:    AuthSession,
    Path(space_id):  Path<String>,
    State(state):    State<AppState>,
    Json(payload):   Json<CreateCompetitionFormPayload>,
) -> impl IntoResponse {
    let mut tmpl = NewCompetitionTemplate {
        space_id:       space_id.clone(),
        name_value:     payload.name.clone(),
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
        Ok(v)  => Some(v),
        Err(_) => {
            tmpl.name_error = Some(
                "Le nom peut contenir lettres, chiffres, espaces et ponctuation courante (100 caractères max).".into(),
            );
            None
        }
    };

    let logo = match CloudinaryImage::try_new(payload.logo_url.clone()) {
        Ok(v)  => Some(v),
        Err(_) => {
            tmpl.logo_error = Some("Veuillez uploader un logo pour votre compétition.".into());
            None
        }
    };

    let (Some(name), Some(logo)) = (name, logo) else {
        return tmpl.into_response();
    };

    let Some(user) = auth_session.user else {
        return Response::builder()
            .header("HX-Redirect", crate::app::auth::routes::path::AUTH_LAYOUT)
            .body(Body::empty())
            .unwrap();
    };

    let admin_ids: Vec<CoachId> = payload.admin_ids
        .iter()
        .filter_map(|id| CoachId::try_new(id).ok())
        .collect();

    let cmd = CreateDraftCompetitionCommand {
        space_id:   sid,
        created_by: user.id,
        name,
        logo,
        admin_ids,
    };

    match execute(
        cmd,
        state.competitions.competition_repository.as_ref(),
        state.competitions.competitions_cache_repository.as_ref(),
        &state.competitions.event_bus,
    ).await {
        Ok(competition_id) => Response::builder()
            .header("HX-Redirect", Routes.new_competition_rules(&space_id, &competition_id.to_string()))
            .body(Body::empty())
            .unwrap(),

        Err(CreateDraftCompetitionError::CompetitionNameAlreadyTaken) => {
            tmpl.name_error = Some("Une compétition avec ce nom existe déjà dans cet espace.".into());
            tmpl.into_response()
        }

        Err(CreateDraftCompetitionError::InvalidAdminId(_)) => {
            tmpl.general_error = Some("Un des administrateurs sélectionnés est invalide.".into());
            tmpl.into_response()
        }

        Err(CreateDraftCompetitionError::Database(_)) => {
            tmpl.general_error = Some("Erreur interne, veuillez réessayer.".into());
            tmpl.into_response()
        }
    }
}

// ── POST phase 2 : sauvegarde des règles ─────────────────────────────────────

pub async fn post_competition_rules(
    Path((space_id, competition_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(rules): Json<CompetitionRules>,
) -> impl IntoResponse {
    let cid = match crate::app::shared_kernel::common_types::CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Identifiant de compétition invalide.").into_response(),
    };

    let cmd = SaveCompetitionRulesCommand { competition_id: cid, rules };

    match execute_save_rules(cmd, state.competitions.competition_repository.as_ref()).await {
        Ok(()) => Response::builder()
            .header("HX-Redirect", Routes.new_competition_structure(&space_id, &competition_id))
            .body(Body::empty())
            .unwrap(),

        Err(SaveCompetitionRulesError::RosterInMultipleTiers { roster, tiers: (t1, t2) }) => {
            let msg = format!("Le roster « {} » est présent dans « {} » et « {} ».", roster, t1, t2);
            (StatusCode::UNPROCESSABLE_ENTITY, msg).into_response()
        }

        Err(SaveCompetitionRulesError::CompetitionNotFound) =>
            (StatusCode::NOT_FOUND, "Compétition introuvable.").into_response(),

        Err(SaveCompetitionRulesError::Database(_)) =>
            (StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne, veuillez réessayer.").into_response(),
    }
}