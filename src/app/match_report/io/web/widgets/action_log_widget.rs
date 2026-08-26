use crate::app::match_report::domain::value_objects::{MatchActionType, TeamSide};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct ActionRowVm {
    pub action_id: String,
    pub turn: i16,
    pub player_name: String,
    pub player_position: String,
    pub action_label: String,
    /// « Nain » plutôt que `DWARF` : le libellé, résolu au catalogue.
    pub hate_label: Option<String>,
    pub action_icon: String,
    pub delete_url: String,
}

#[derive(Template)]
#[template(path = "action-log-widget.html")]
pub struct ActionLogTemplate {
    pub actions: Vec<ActionRowVm>,
}

impl IntoResponse for ActionLogTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_action_log_step3(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    render_action_log(space_id, mr_id, TeamSide::Home, state).await
}

pub async fn get_action_log_step4(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    render_action_log(space_id, mr_id, TeamSide::Away, state).await
}

async fn render_action_log(
    space_id: String,
    mr_id: String,
    side: TeamSide,
    state: AppState,
) -> Response {
    let rows = match state
        .match_report
        .match_report_repo
        .find_actions_by_match_and_side(&mr_id, side)
        .await
    {
        Ok(r) => r,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let routes = AppRoutes::default();
    let actions = rows
        .into_iter()
        .map(|r| {
            let (action_label, action_icon) = parse_action(&r.action_json);
            let hate_label =
                libelle_de_haine(&r.action_json, state.match_report.keyword_catalog.as_ref());
            ActionRowVm {
                delete_url: routes
                    .match_report
                    .delete_action(&space_id, &mr_id, &r.action_id),
                action_id: r.action_id,
                turn: r.turn_number,
                player_name: r.player_display_name,
                player_position: r.player_position,
                action_label,
                action_icon,
                hate_label,
            }
        })
        .collect();
    ActionLogTemplate { actions }.into_response()
}

fn parse_action(json: &serde_json::Value) -> (String, String) {
    if let Ok(action) = serde_json::from_value::<MatchActionType>(json.clone()) {
        return (
            action_type_label(&action),
            action_type_icon(&action).to_string(),
        );
    }
    ("Action inconnue".to_string(), "❓".to_string())
}

/// Le libellé du mot-clef haï, s'il y en a un.
///
/// Un uid que le catalogue ne connaît plus est **affiché tel quel** plutôt
/// qu'escamoté : une Haine qui disparaît de l'écran sans un mot serait le genre
/// de silence que cette série corrige.
fn libelle_de_haine(
    json: &serde_json::Value,
    catalogue: &dyn crate::app::match_report::ports::IKeywordCatalogPort,
) -> Option<String> {
    let MatchActionType::Blesse {
        hatred: Some(mot), ..
    } = serde_json::from_value::<MatchActionType>(json.clone()).ok()?
    else {
        return None;
    };
    let uid = mot.to_string();
    Some(
        catalogue
            .find_hateable(&uid)
            .map(|k| k.label)
            .unwrap_or(uid),
    )
}

fn action_type_label(action: &MatchActionType) -> String {
    match action {
        MatchActionType::Touchdown => "Touchdown".into(),
        MatchActionType::Passe => "Passe".into(),
        MatchActionType::Interception => "Interception".into(),
        MatchActionType::Agression => "Agression".into(),
        MatchActionType::Lancer => "Lancer".into(),
        MatchActionType::Sortie => "Sortie".into(),
        MatchActionType::Mvp => "MVP".into(),
        MatchActionType::Blesse { .. } => "Blessé".into(),
    }
}

fn action_type_icon(action: &MatchActionType) -> &'static str {
    match action {
        MatchActionType::Touchdown => "🏈",
        MatchActionType::Passe => "🎯",
        MatchActionType::Interception => "🛡️",
        MatchActionType::Agression => "💥",
        MatchActionType::Lancer => "🤾",
        MatchActionType::Sortie => "🩸",
        MatchActionType::Mvp => "⭐",
        MatchActionType::Blesse { .. } => "🚑",
    }
}
