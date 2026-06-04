use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

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

    let allowed: Option<std::collections::HashSet<&str>> = if !params.roster_id.is_empty() {
        ref_repo
            .find_team_by_uid(&params.roster_id)
            .map(|t| t.special_rules.iter().map(String::as_str).collect())
    } else {
        None
    };

    let rules: Vec<SpecialRuleVm> = ref_repo
        .list_special_rules()
        .iter()
        .filter(|r| allowed.as_ref().map_or(true, |set| set.contains(r.uid.as_str())))
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
