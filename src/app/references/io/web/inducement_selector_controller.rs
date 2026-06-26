use crate::app::references::domain::port::IReferenceRepository;
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct InducementSelectorParams {
    #[serde(default)]
    pub allowed_inducement_uids: String,
    #[serde(default)]
    pub allowed_star_player_uids: String,
    #[serde(default)]
    pub roster_id: String,
    #[serde(default)]
    pub instance_id: String,
    #[serde(default)]
    pub selected: String,
}

pub struct InducementSelectorItem {
    pub uid: String,
    pub name: String,
    pub description: String,
    pub unit_cost: u32,
    pub max_qty: u8,
    pub is_common: bool,
    pub initial_qty: u8,
}

pub struct StarPlayerSelectorItem {
    pub uid: String,
    pub name: String,
    pub rosters_label: String,
    pub cost: u32,
    pub ma: u8,
    pub st: u8,
    pub ag: String,
    pub pa: String,
    pub av: String,
    pub skills: Vec<String>,
    pub special_ability_name: String,
    pub special_ability_description: String,
    pub initial_qty: u8,
}

#[derive(Serialize)]
struct JsItem {
    uid: String,
    name: String,
    max_qty: u8,
    unit_cost: u32,
    qty: u8,
}

pub async fn inducement_selector_controller(
    State(state): State<AppState>,
    Query(params): Query<InducementSelectorParams>,
) -> impl IntoResponse {
    let repo = state.references.repository.as_ref();
    let initial_qtys = parse_selected(&params.selected);
    let (common_items, special_items) = build_inducement_items(&params, repo, &initial_qtys);
    let star_player_items = build_star_player_items(&params, repo, &initial_qtys);
    let items_json = build_items_json(&common_items, &special_items, &star_player_items);
    InducementSelectorTemplate {
        routes: AppRoutes::default(),
        common_items,
        special_items,
        star_player_items,
        instance_id: params.instance_id,
        items_json,
    }
    .into_response()
}

fn build_items_json(
    common: &[InducementSelectorItem],
    special: &[InducementSelectorItem],
    stars: &[StarPlayerSelectorItem],
) -> String {
    let all: Vec<JsItem> = common
        .iter()
        .chain(special.iter())
        .map(|i| JsItem { uid: i.uid.clone(), name: i.name.clone(), max_qty: i.max_qty, unit_cost: i.unit_cost, qty: i.initial_qty })
        .chain(stars.iter().map(|s| JsItem { uid: s.uid.clone(), name: s.name.clone(), max_qty: 1, unit_cost: s.cost, qty: s.initial_qty }))
        .collect();
    serde_json::to_string(&all).unwrap_or_else(|_| "[]".to_string())
}

fn parse_selected(selected: &str) -> HashMap<String, u8> {
    selected
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, ':');
            let uid = parts.next()?.to_string();
            let qty: u8 = parts.next()?.parse().ok()?;
            Some((uid, qty))
        })
        .collect()
}

fn build_inducement_items(
    params: &InducementSelectorParams,
    repo: &dyn IReferenceRepository,
    initial_qtys: &HashMap<String, u8>,
) -> (Vec<InducementSelectorItem>, Vec<InducementSelectorItem>) {
    let allowed: Vec<&str> = params
        .allowed_inducement_uids
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    let mut common = vec![];
    let mut special = vec![];
    for uid in &allowed {
        let Some(ind) = repo.find_inducement_by_uid(uid) else { continue };
        if !params.roster_id.is_empty()
            && !ind.restricted_to.is_empty()
            && !ind.restricted_to.contains(&params.roster_id)
        {
            continue;
        }
        let item = InducementSelectorItem {
            uid: ind.uid.clone(),
            name: ind.name.clone(),
            description: ind.description.clone(),
            unit_cost: ind.cost,
            max_qty: ind.max_quantity as u8,
            is_common: is_common_category(&ind.category),
            initial_qty: *initial_qtys.get(&ind.uid).unwrap_or(&0),
        };
        if item.is_common {
            common.push(item);
        } else {
            special.push(item);
        }
    }
    (common, special)
}

fn build_star_player_items(
    params: &InducementSelectorParams,
    repo: &dyn IReferenceRepository,
    initial_qtys: &HashMap<String, u8>,
) -> Vec<StarPlayerSelectorItem> {
    let allowed: Vec<&str> = params
        .allowed_star_player_uids
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    allowed
        .iter()
        .filter_map(|uid| {
            let sp = repo.find_star_player_by_uid(uid)?;
            if !params.roster_id.is_empty()
                && !sp.available_for_rosters.is_empty()
                && !sp.available_for_rosters.contains(&params.roster_id)
            {
                return None;
            }
            let rosters_label = if sp.available_for_rosters.is_empty() {
                "Toutes les équipes".to_string()
            } else {
                sp.available_for_rosters.join(", ")
            };
            Some(StarPlayerSelectorItem {
                uid: sp.uid.clone(),
                name: sp.name.clone(),
                rosters_label,
                cost: sp.cost,
                ma: sp.ma,
                st: sp.st,
                ag: sp.ag.clone(),
                pa: sp.pa.clone(),
                av: sp.av.clone(),
                skills: sp.skills.clone(),
                special_ability_name: sp.special_ability_name.clone(),
                special_ability_description: sp.special_ability_description.clone(),
                initial_qty: *initial_qtys.get(&sp.uid).unwrap_or(&0),
            })
        })
        .collect()
}

fn is_common_category(category: &str) -> bool {
    matches!(category, "COMMON" | "INFAMOUS_STAFF" | "WIZARD" | "BIASED_REFEREE")
}

#[derive(Template)]
#[template(path = "widgets/inducement-selector.html")]
pub struct InducementSelectorTemplate {
    pub routes: AppRoutes,
    pub common_items: Vec<InducementSelectorItem>,
    pub special_items: Vec<InducementSelectorItem>,
    pub star_player_items: Vec<StarPlayerSelectorItem>,
    pub instance_id: String,
    pub items_json: String,
}

impl IntoResponse for InducementSelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
