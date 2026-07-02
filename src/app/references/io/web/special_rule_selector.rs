use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// Les UIDs concrets que FAVOURED_OF_CHOOSE_* doit exposer
const FIVE_CHAOS_GODS: [&str; 5] = [
    "FAVOURED_OF_KHORNE",
    "FAVOURED_OF_NURGLE",
    "FAVOURED_OF_SLAANESH",
    "FAVOURED_OF_TZEENTCH",
    "FAVOURED_OF_UNDIVIDED",
];

#[derive(Deserialize)]
pub struct SpecialRuleSelectorParams {
    #[serde(default)]
    pub selected: String,
    pub on_select: String,
    #[serde(default)]
    pub roster_id: String,
}

pub struct SpecialRuleVm {
    pub uid:         String,
    pub label:       String,
    pub is_selected: bool,
}

#[derive(Template)]
#[template(path = "special-rule-selector-fragment.html")]
pub struct SpecialRuleSelectorTemplate {
    pub rules:          Vec<SpecialRuleVm>,
    pub selected_uid:   String,
    pub selected_label: Option<String>,
    pub on_select:      String,
}

impl IntoResponse for SpecialRuleSelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn special_rule_selector(
    Query(params): Query<SpecialRuleSelectorParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let ref_repo = state.references.repository.as_ref();

    let team_rules: Vec<String> = if !params.roster_id.is_empty() {
        ref_repo
            .find_team_by_uid(&params.roster_id)
            .map(|t| t.special_rules.clone())
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Seuls les UIDs FAVOURED_OF_CHOOSE_* déclenchent un choix utilisateur
    let has_choose = team_rules
        .iter()
        .any(|r| r.starts_with("FAVOURED_OF_CHOOSE_"));

    if !has_choose {
        return SpecialRuleSelectorTemplate {
            rules: vec![],
            selected_uid: params.selected,
            selected_label: None,
            on_select: params.on_select,
        }
        .into_response();
    }

    // Les deux placeholders existants s'expandent vers les 5 dieux du Chaos
    // L'ordre est déterminé par FIVE_CHAOS_GODS, pas par le repository
    let all_rules = ref_repo.list_special_rules();
    let rules: Vec<SpecialRuleVm> = FIVE_CHAOS_GODS
        .iter()
        .filter_map(|uid| all_rules.iter().find(|r| r.uid == *uid))
        .map(|r| SpecialRuleVm {
            is_selected: r.uid == params.selected,
            uid:         r.uid.clone(),
            label:       r.label.clone(),
        })
        .collect();

    let selected_label = rules.iter().find(|r| r.is_selected).map(|r| r.label.clone());

    SpecialRuleSelectorTemplate {
        rules,
        selected_uid: params.selected,
        selected_label,
        on_select: params.on_select,
    }
    .into_response()
}
