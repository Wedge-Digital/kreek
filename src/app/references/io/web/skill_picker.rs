use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SkillPickerParams {
    pub roster_line_id: String,
    #[serde(default)]
    pub spp_remaining: u8,
    #[serde(default)]
    pub acquired: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub filters: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRowVm {
    pub id:             String,
    pub name:           String,
    pub desc:           String,
    pub category:       String,
    pub category_label: String,
    pub category_css:   String,
    pub is_primary:     bool,
    pub is_elite:       bool,
    pub cost_chosen:    u8,
    pub cost_random:    u8,
    pub is_acquired:    bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChosenCostVm {
    pub primary:   u8,
    pub secondary: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingVm {
    pub chosen:       ChosenCostVm,
    pub chosen_elite: ChosenCostVm,
    pub random:       u8,
    pub random_elite: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryFilterVm {
    pub id:           String,
    pub label:        String,
    pub is_primary:   bool,
}

#[derive(Template)]
#[template(path = "skill-picker-fragment.html")]
pub struct SkillPickerTemplate {
    pub skills_json:          String,
    pub categories_json:      String,
    pub pricing_json:         String,
    pub spp_remaining:        u8,
    pub initial_mode_json:    String,
    pub initial_filters_json: String,
}

impl IntoResponse for SkillPickerTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn category_css(uid: &str) -> &'static str {
    match uid {
        "GENERAL"  => "type-general",
        "STRENGTH" => "type-strength",
        "AGILITY"  => "type-agility",
        "PASSING"  => "type-passing",
        "MUTATION" => "type-mutation",
        _          => "type-general",
    }
}

pub async fn skill_picker(
    Query(params): Query<SkillPickerParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let repo = &state.references.repository;

    let Some(position) = repo.find_position_by_uid(&params.roster_line_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let acquired_set: std::collections::HashSet<&str> = params
        .acquired
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();

    let primary_set: std::collections::HashSet<&str> =
        position.primary_access.iter().map(|s| s.as_str()).collect();

    let accessible: std::collections::HashSet<&str> = position
        .primary_access
        .iter()
        .chain(position.secondary_access.iter())
        .map(|s| s.as_str())
        .collect();

    // Pricing: coût basé sur accès primaire/secondaire, pas sur skill_type
    let pricing = repo
        .skill_cost_matrix()
        .iter()
        .find(|l| l.level == 1)
        .expect("level 1 must exist");

    let mut categories: Vec<CategoryFilterVm> = repo
        .list_skill_categories()
        .iter()
        .filter(|c| accessible.contains(c.id.as_str()))
        .map(|c| CategoryFilterVm {
            is_primary: primary_set.contains(c.id.as_str()),
            id:         c.id.clone(),
            label:      c.label.clone(),
        })
        .collect();
    categories.sort_by(|a, b| a.label.cmp(&b.label));

    let pricing_vm = PricingVm {
        chosen:       ChosenCostVm { primary: pricing.chosen.primary, secondary: pricing.chosen.secondary },
        chosen_elite: ChosenCostVm {
            primary:   pricing.chosen_for(true).primary,
            secondary: pricing.chosen_for(true).secondary,
        },
        random:       pricing.random,
        random_elite: pricing.random_for(true),
    };

    let skills: Vec<SkillRowVm> = repo
        .list_skills()
        .iter()
        .filter(|s| accessible.contains(s.category.as_str()))
        .map(|s| {
            let is_primary  = primary_set.contains(s.category.as_str());
            let is_elite    = s.skill_type == "Élite";
            let costs       = pricing.chosen_for(is_elite);
            let cost_chosen = if is_primary { costs.primary } else { costs.secondary };
            let cost_random = pricing.random_for(is_elite);
            let is_acquired = acquired_set.contains(s.uid.as_str());
            let cat_label   = repo
                .list_skill_categories()
                .iter()
                .find(|c| c.id == s.category)
                .map(|c| c.label.clone())
                .unwrap_or_else(|| s.category.clone());

            SkillRowVm {
                is_acquired,
                is_primary,
                is_elite,
                cost_chosen,
                cost_random,
                id:             s.uid.clone(),
                name:           s.name.clone(),
                desc:           s.description.clone(),
                category:       s.category.clone(),
                category_label: cat_label,
                category_css:   category_css(&s.category).to_string(),
            }
        })
        .collect();

    let skills_json     = serde_json::to_string(&skills).unwrap_or_default();
    let categories_json = serde_json::to_string(&categories).unwrap_or_default();
    let pricing_json    = serde_json::to_string(&pricing_vm).unwrap_or_default();

    let mode = if params.mode == "random" { "random" } else { "chosen" };
    let initial_mode_json = serde_json::to_string(mode).unwrap_or_default();

    let active_filters: Vec<&str> = params.filters
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    let initial_filters_json = serde_json::to_string(&active_filters).unwrap_or_default();

    SkillPickerTemplate {
        skills_json,
        categories_json,
        pricing_json,
        spp_remaining:        params.spp_remaining,
        initial_mode_json,
        initial_filters_json,
    }
    .into_response()
}
