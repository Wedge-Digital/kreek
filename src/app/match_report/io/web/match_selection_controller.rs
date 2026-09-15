use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::MatchReportOrigin;
use crate::app::match_report::use_cases::match_report_access_service::{
    est_administrateur, AccesRapportDeps,
};
use crate::app::match_report::use_cases::{
    create_match_report_use_case, update_match_selection_use_case,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "match-selection.html")]
pub struct MatchSelectionTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub widget_url: String,
    pub team_widget_url: String,
    pub is_prefilled: bool,
    pub error_message: Option<String>,
    pub form_action: String,
    /// La sélection est figée (carte 550) : les deux widgets cèdent la place
    /// aux quatre noms en clair. `None` quand elle reste modifiable.
    pub lecture_seule: Option<SelectionFigeeVm>,
}

/// Ce que la phase 1 montre quand elle ne se modifie plus.
///
/// **Des noms, pas des identifiants.** Un ULID n'apprend rien au coach, et le
/// remplacer par un widget désactivé aurait obligé deux autres BCs à porter un
/// mode dont ils n'ont besoin nulle part ailleurs.
pub struct SelectionFigeeVm {
    pub competition_name: String,
    pub season_name: String,
    pub round_name: String,
    pub home_team_name: String,
    pub away_team_name: String,
}

impl IntoResponse for MatchSelectionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn build_widget_url(space_id: &str) -> String {
    AppRoutes::default()
        .competitions
        .competition_widget(space_id)
        + "?show_rounds=true"
}

fn build_widget_url_prefilled(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    round_id: &str,
) -> String {
    format!(
        "{}?show_rounds=true&competition_id={}&season_id={}&round_id={}",
        AppRoutes::default()
            .competitions
            .competition_widget(space_id),
        competition_id,
        season_id,
        round_id,
    )
}

fn build_team_widget_url(space_id: &str) -> String {
    AppRoutes::default().teams.team_selection_widget(space_id)
}

fn build_team_widget_url_prefilled(
    space_id: &str,
    season_id: &str,
    home_id: &str,
    away_id: &str,
) -> String {
    format!(
        "{}?season_id={}&selected_home={}&selected_away={}",
        AppRoutes::default().teams.team_selection_widget(space_id),
        season_id,
        home_id,
        away_id,
    )
}

// ── Handlers GET ─────────────────────────────────────────────────────────────

pub async fn from_pairing(
    Path((space_id, pairing_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mr_id = match state
        .match_report
        .match_report_repo
        .find_id_by_pairing(&pairing_id)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("from_pairing find_id_by_pairing {pairing_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let url = AppRoutes::default()
        .match_report
        .edit_match_report(&space_id, &mr_id);
    Redirect::to(&url).into_response()
}

/// La sélection est-elle figée pour cet utilisateur, et si oui que montrer ?
///
/// `Some(..)` seulement quand **les deux** conditions tiennent : la compétition
/// interdit les matchs hors calendrier, et l'utilisateur n'en est pas
/// administrateur. Un admin garde ses widgets, et une compétition qui autorise
/// ne fige personne.
///
/// Les quatre noms sont résolus par les ports qui existent déjà —
/// `find_round_context` pour la compétition et la journée, `find_team_info` pour
/// les deux équipes. Un nom manquant devient l'identifiant : mieux qu'une case
/// vide, et le problème se voit.
#[allow(clippy::too_many_arguments)]
async fn selection_figee(
    state: &AppState,
    user: &crate::app::auth::domain::user::User,
    space_id: &str,
    comp_id: &str,
    season_id: &str,
    round_id: &str,
    home_id: &str,
    away_id: &str,
) -> Option<SelectionFigeeVm> {
    let comp = state.match_report.competition_data.as_ref();
    if comp.autorise_hors_calendrier(season_id).await {
        return None;
    }

    let deps = AccesRapportDeps::from_state(state);
    if est_administrateur(&deps, user, space_id, comp_id).await {
        return None;
    }

    let contexte = comp.find_round_context(season_id, round_id).await;
    let teams = state.match_report.team_data.as_ref();
    let (home, away) = tokio::join!(teams.find_team_info(home_id), teams.find_team_info(away_id));

    Some(SelectionFigeeVm {
        competition_name: contexte
            .as_ref()
            .map(|c| c.competition_name.clone())
            .unwrap_or_else(|| comp_id.to_string()),
        season_name: contexte
            .as_ref()
            .map(|c| c.season_name.clone())
            .unwrap_or_else(|| season_id.to_string()),
        round_name: contexte
            .as_ref()
            .map(|c| c.round_name.clone())
            .unwrap_or_else(|| round_id.to_string()),
        home_team_name: home
            .map(|t| t.team_name)
            .unwrap_or_else(|| home_id.to_string()),
        away_team_name: away
            .map(|t| t.team_name)
            .unwrap_or_else(|| away_id.to_string()),
    })
}

pub async fn new_match_report(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let form_action = AppRoutes::default()
        .match_report
        .new_match_report(&space_id);

    MatchSelectionTemplate {
        app_routes: Default::default(),
        widget_url: build_widget_url(&space_id),
        team_widget_url: build_team_widget_url(&space_id),
        space_id,
        is_prefilled: false,
        error_message: None,
        form_action,
        // Jamais figée : la garde de `create_match_report` interdit déjà d'y
        // arriver quand la compétition refuse le hors-calendrier.
        lecture_seule: None,
    }
    .into_response()
}

pub async fn edit_match_report(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let mr_state = match state
        .match_report
        .match_report_repo
        .find_by_id(&match_report_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("edit_match_report find_by_id {match_report_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match mr_state {
        MatchReportState::Draft(draft) => {
            let comp_id = draft.competition_id.to_string();
            let season_id = draft.season_id.to_string();
            let round_id = draft.round_id.to_string();
            let home_id = draft.home_team_id.to_string();
            let away_id = draft.away_team_id.to_string();

            let form_action = AppRoutes::default()
                .match_report
                .edit_match_report(&space_id, &match_report_id);

            let lecture_seule = selection_figee(
                &state, &user, &space_id, &comp_id, &season_id, &round_id, &home_id, &away_id,
            )
            .await;

            MatchSelectionTemplate {
                app_routes: Default::default(),
                widget_url: build_widget_url_prefilled(&space_id, &comp_id, &season_id, &round_id),
                team_widget_url: build_team_widget_url_prefilled(
                    &space_id, &season_id, &home_id, &away_id,
                ),
                space_id,
                is_prefilled: true,
                error_message: None,
                form_action,
                lecture_seule,
            }
            .into_response()
        }
        MatchReportState::PreMatch(_pm) => {
            let url = format!("/app/{}/match-report/{}/step2", space_id, match_report_id);
            Redirect::to(&url).into_response()
        }
        MatchReportState::ReadyToPublish(_) => {
            let url = AppRoutes::default()
                .match_report
                .step5(&space_id, &match_report_id);
            Redirect::to(&url).into_response()
        }
        MatchReportState::Cancelled(_) => StatusCode::GONE.into_response(),
        MatchReportState::Published(_) => StatusCode::CONFLICT.into_response(),
    }
}

// ── POST form ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateMatchReportForm {
    pub competition_id: String,
    pub season_id: String,
    pub round_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

// ── Handlers POST ────────────────────────────────────────────────────────────

pub async fn create_match_report(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<CreateMatchReportForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    // Carte 550 — la garde serveur. Retirer les entrées de menu cache la
    // fonction ; ça n'empêche pas d'appeler la route à la main, et sans ce
    // refus le réglage serait décoratif.
    //
    // La question porte sur la saison **choisie dans le formulaire**, et non sur
    // l'espace : c'est le seul endroit où elle a une réponse exacte. Le menu,
    // lui, cache l'entrée dès qu'une compétition de l'espace interdit — les deux
    // règles diffèrent, et c'est voulu.
    if !state
        .match_report
        .competition_data
        .autorise_hors_calendrier(&form.season_id)
        .await
    {
        tracing::info!(
            season_id = %form.season_id,
            "création manuelle refusée : la saison interdit les matchs hors calendrier"
        );
        return StatusCode::FORBIDDEN.into_response();
    }

    let cmd = create_match_report_use_case::CreateMatchReportCommand {
        space_id: match SpaceId::try_new(&space_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        competition_id: match CompetitionId::try_new(&form.competition_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        season_id: match SeasonId::try_new(&form.season_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        round_id: match RoundId::try_new(&form.round_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        home_team_id: match TeamId::try_new(&form.home_team_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        away_team_id: match TeamId::try_new(&form.away_team_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        created_by: user.id,
        origin: MatchReportOrigin::Manual,
        pairing_id: None,
    };

    match create_match_report_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        &state.app_event_bus,
    )
    .await
    {
        Ok(mr_id) => {
            let url = AppRoutes::default()
                .match_report
                .edit_match_report(&space_id, &mr_id.to_string());
            Redirect::to(&url).into_response()
        }
        Err(create_match_report_use_case::CreateMatchReportError::SameTeam) => {
            StatusCode::BAD_REQUEST.into_response()
        }
        Err(e) => {
            tracing::error!("create_match_report: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Les deux équipes qui partent réellement dans la commande.
///
/// Celles du formulaire, sauf quand la sélection est figée pour cet utilisateur
/// (carte 550) : ce sont alors celles déjà enregistrées. Le rapport doit être en
/// `Draft` pour qu'on en soit là — les autres états ne passent pas par ce POST.
async fn equipes_retenues(
    state: &AppState,
    user: &crate::app::auth::domain::user::User,
    space_id: &str,
    match_report_id: &str,
    form: &CreateMatchReportForm,
) -> (String, String) {
    let saisies = (form.home_team_id.clone(), form.away_team_id.clone());

    let Ok(Some(MatchReportState::Draft(draft))) = state
        .match_report
        .match_report_repo
        .find_by_id(match_report_id)
        .await
    else {
        return saisies;
    };

    let figee = selection_figee(
        state,
        user,
        space_id,
        &draft.competition_id.to_string(),
        &draft.season_id.to_string(),
        &draft.round_id.to_string(),
        &draft.home_team_id.to_string(),
        &draft.away_team_id.to_string(),
    )
    .await;

    match figee {
        Some(_) => (
            draft.home_team_id.to_string(),
            draft.away_team_id.to_string(),
        ),
        None => saisies,
    }
}

pub async fn update_match_selection(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<CreateMatchReportForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    // Carte 550 — la garde **n'interdit rien, elle ignore**. Un non-admin sur une
    // compétition qui refuse le hors-calendrier confirme la sélection telle
    // qu'elle est en base ; les deux équipes envoyées par le formulaire sont
    // écartées.
    //
    // Ignorer plutôt que refuser : le POST reste le seul chemin de `Draft` vers
    // `PreMatch`, donc le refuser empêcherait le coach de commencer son rapport.
    // Et un corps trafiqué produit alors exactement ce qu'aurait produit un
    // corps honnête — rien à mettre en mots à l'écran.
    let (home_saisi, away_saisi) =
        equipes_retenues(&state, &user, &space_id, &match_report_id, &form).await;

    let cmd = update_match_selection_use_case::UpdateMatchSelectionCommand {
        match_report_id: match MatchReportId::try_new(&match_report_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        home_team_id: match TeamId::try_new(&home_saisi) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        away_team_id: match TeamId::try_new(&away_saisi) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        confirmed_by: user.id,
    };

    match update_match_selection_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        &state.app_event_bus,
    )
    .await
    {
        Ok(_mr_id) => {
            let url = format!("/app/{}/match-report/{}/step2", space_id, match_report_id);
            Redirect::to(&url).into_response()
        }
        Err(update_match_selection_use_case::UpdateMatchSelectionError::SameTeam) => {
            StatusCode::BAD_REQUEST.into_response()
        }
        Err(update_match_selection_use_case::UpdateMatchSelectionError::TeamNotAvailable(tid)) => {
            tracing::warn!("update_match_selection: team {tid} not available");
            StatusCode::CONFLICT.into_response()
        }
        Err(e) => {
            tracing::error!("update_match_selection: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
