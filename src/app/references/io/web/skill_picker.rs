use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SkillPickerParams {
    pub roster_line_id: String,
    #[serde(default)]
    pub spp: u8,
    #[serde(default)]
    pub acquired: String,
    pub on_acquire: String,
    pub on_cancel: String,
}

pub struct SkillRowVm {
    pub uid: String,
    pub name: String,
    pub description: String,
    pub category_uid: String,
    pub category_label: String,
    pub category_css: String,
    pub is_elite: bool,
    pub cost_chosen: u8,
    pub cost_random: u8,
    pub is_acquired: bool,
    pub is_affordable_chosen: bool,
    pub is_affordable_random: bool,
    pub is_primary: bool,
}

pub struct CategoryFilterVm {
    pub uid: String,
    pub label: String,
    pub is_primary: bool,
    pub is_secondary: bool,
}

#[derive(Template)]
#[template(path = "skill-picker-fragment.html")]
pub struct SkillPickerTemplate {
    pub skills: Vec<SkillRowVm>,
    pub categories: Vec<CategoryFilterVm>,
    pub spp: u8,
    pub on_acquire: String,
    pub on_cancel: String,
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
        "GENERAL" => "type-general",
        "STRENGTH" => "type-strength",
        "AGILITY" => "type-agility",
        "PASSING" => "type-passing",
        "MUTATION" => "type-mutation",
        _ => "type-general",
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

    let accessible: std::collections::HashSet<&str> = position
        .primary_access
        .iter()
        .chain(position.secondary_access.iter())
        .map(|s| s.as_str())
        .collect();

    let primary_set: std::collections::HashSet<&str> =
        position.primary_access.iter().map(|s| s.as_str()).collect();

    let mut categories: Vec<CategoryFilterVm> = repo
        .list_skill_categories()
        .iter()
        .filter(|c| accessible.contains(c.id.as_str()))
        .map(|c| CategoryFilterVm {
            is_primary: primary_set.contains(c.id.as_str()),
            is_secondary: !primary_set.contains(c.id.as_str()),
            uid: c.id.clone(),
            label: c.label.clone(),
        })
        .collect();
    categories.sort_by(|a, b| a.label.cmp(&b.label));

    let skills: Vec<SkillRowVm> = repo
        .list_skills()
        .iter()
        .filter(|s| accessible.contains(s.category.as_str()))
        .map(|s| {
            let is_elite = s.skill_type == "Élite";
            let cost_chosen = if is_elite { 6 } else { 3 };
            let cost_random = if is_elite { 4 } else { 2 };
            let is_acquired = acquired_set.contains(s.uid.as_str());
            let cat_label = repo
                .list_skill_categories()
                .iter()
                .find(|c| c.id == s.category)
                .map(|c| c.label.clone())
                .unwrap_or_else(|| s.category.clone());

            SkillRowVm {
                is_affordable_chosen: !is_acquired && params.spp >= cost_chosen,
                is_affordable_random: !is_acquired && params.spp >= cost_random,
                is_primary: primary_set.contains(s.category.as_str()),
                is_acquired,
                is_elite,
                cost_chosen,
                cost_random,
                uid: s.uid.clone(),
                name: s.name.clone(),
                description: s.description.clone(),
                category_uid: s.category.clone(),
                category_label: cat_label,
                category_css: category_css(&s.category).to_string(),
            }
        })
        .collect();

    SkillPickerTemplate {
        skills,
        categories,
        spp: params.spp,
        on_acquire: params.on_acquire,
        on_cancel: params.on_cancel,
    }
    .into_response()
}
