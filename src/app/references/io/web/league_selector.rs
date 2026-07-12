use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LeagueSelectorParams {
    #[serde(default)]
    pub selected: String,
    pub on_select: String,
    #[serde(default)]
    pub roster_id: String,
}

pub struct LeagueSelectorVm {
    pub uid: String,
    pub label: String,
    pub is_selected: bool,
}

#[derive(Template)]
#[template(path = "league-selector-fragment.html")]
pub struct LeagueSelectorTemplate {
    pub leagues: Vec<LeagueSelectorVm>,
    pub selected_uid: String,
    pub selected_label: Option<String>,
    pub on_select: String,
}

impl IntoResponse for LeagueSelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn league_selector(
    Query(params): Query<LeagueSelectorParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let ref_repo = state.references.repository.as_ref();

    let league_uids: Vec<String> = if !params.roster_id.is_empty() {
        ref_repo
            .find_team_by_uid(&params.roster_id)
            .map(|t| t.leagues.clone())
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Construire les VMs dans l'ordre des ligues du roster
    let all_leagues = ref_repo.list_leagues();
    let leagues: Vec<LeagueSelectorVm> = league_uids
        .iter()
        .filter_map(|uid| all_leagues.iter().find(|l| &l.uid == uid))
        .map(|l| LeagueSelectorVm {
            is_selected: l.uid == params.selected,
            uid: l.uid.clone(),
            label: l.label.clone(),
        })
        .collect();

    let selected_label = leagues
        .iter()
        .find(|l| l.is_selected)
        .map(|l| l.label.clone());

    LeagueSelectorTemplate {
        leagues,
        selected_uid: params.selected,
        selected_label,
        on_select: params.on_select,
    }
    .into_response()
}
