use crate::app::match_report::domain::value_objects::TeamSide;
use crate::app::match_report::use_cases::hate_keywords_service::{self, HateKeywordChoices};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ActionPanelParams {
    pub turn: u8,
    pub player_id: String,
    pub player_type: String,
}

#[derive(Template)]
#[template(path = "action-panel-widget.html")]
pub struct ActionPanelTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub team_side: String,
    pub turn: u8,
    pub player_id: String,
    pub player_type: String,
    pub post_url: String,
    /// Les mots-clefs du roster d'en face, puis tous les autres. Ils voyagent
    /// **en HTML** : le filtre travaille sur le DOM déjà rendu, ce qui évite un
    /// endpoint, un JSON, et toute latence au moment du clic.
    pub hate_in_roster: Vec<HateKeywordVm>,
    pub hate_others: Vec<HateKeywordVm>,
}

pub struct HateKeywordVm {
    pub uid: String,
    pub label: String,
}

impl HateKeywordVm {
    fn all_from(choices: HateKeywordChoices) -> (Vec<Self>, Vec<Self>) {
        let vers_vm = |k: crate::app::match_report::ports::KeywordDto| Self {
            uid: k.uid,
            label: k.label,
        };
        (
            choices
                .in_opponent_roster
                .into_iter()
                .map(vers_vm)
                .collect(),
            choices.others.into_iter().map(vers_vm).collect(),
        )
    }
}

impl IntoResponse for ActionPanelTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_action_panel_step3(
    Path((space_id, mr_id)): Path<(String, String)>,
    Query(params): Query<ActionPanelParams>,
    State(state): State<AppState>,
) -> Response {
    rendre_panneau(space_id, mr_id, TeamSide::Home, params, state).await
}

pub async fn get_action_panel_step4(
    Path((space_id, mr_id)): Path<(String, String)>,
    Query(params): Query<ActionPanelParams>,
    State(state): State<AppState>,
) -> Response {
    rendre_panneau(space_id, mr_id, TeamSide::Away, params, state).await
}

async fn rendre_panneau(
    space_id: String,
    mr_id: String,
    side: TeamSide,
    params: ActionPanelParams,
    state: AppState,
) -> Response {
    let routes = AppRoutes::default();
    let post_url = match side {
        TeamSide::Home => routes.match_report.step3_post_action(&space_id, &mr_id),
        TeamSide::Away => routes.match_report.step4_post_action(&space_id, &mr_id),
    };
    let (hate_in_roster, hate_others) = choix_de_haine(&mr_id, side, &state).await;
    ActionPanelTemplate {
        app_routes: routes,
        space_id,
        match_report_id: mr_id,
        team_side: match side {
            TeamSide::Home => "home".into(),
            TeamSide::Away => "away".into(),
        },
        turn: params.turn,
        player_id: params.player_id,
        player_type: params.player_type,
        post_url,
        hate_in_roster,
        hate_others,
    }
    .into_response()
}

/// Le joueur blessé hait l'espèce d'en face : le roster à consulter est celui de
/// l'**adversaire**.
///
/// Un rapport introuvable rend deux listes vides plutôt qu'une erreur : la
/// section de Haine disparaît, le reste du panneau fonctionne, et la ligne de
/// journal dit pourquoi.
async fn choix_de_haine(
    mr_id: &str,
    side: TeamSide,
    state: &AppState,
) -> (Vec<HateKeywordVm>, Vec<HateKeywordVm>) {
    let equipes = state
        .match_report
        .match_report_repo
        .find_team_ids(mr_id)
        .await;
    let adverse = match equipes {
        Ok(Some((domicile, exterieur))) => match side {
            TeamSide::Home => exterieur,
            TeamSide::Away => domicile,
        },
        autre => {
            tracing::warn!(match_report_id = %mr_id, "panneau d'action sans équipes : {autre:?}");
            return (vec![], vec![]);
        }
    };
    let choices = hate_keywords_service::choix_de_haine(
        &adverse,
        state.match_report.keyword_catalog.as_ref(),
        state.match_report.team_data.as_ref(),
    )
    .await;
    HateKeywordVm::all_from(choices)
}
