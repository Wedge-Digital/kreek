use crate::app::competitions::routes::Routes as CompetitionRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};

// ── Widget principal ─────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "competition-widget.html")]
pub struct CompetitionWidgetTemplate {
    pub routes: CompetitionRoutes,
    pub space_id: String,
    pub show_detail: bool,
    pub show_rounds: bool,
    pub selected_competition_id: String,
    pub selected_season_id: String,
    pub selected_round_id: String,
}

impl IntoResponse for CompetitionWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct CompetitionWidgetQuery {
    #[serde(default)]
    pub show_detail: bool,
    #[serde(default)]
    pub show_rounds: bool,
    pub competition_id: Option<String>,
    pub season_id: Option<String>,
    pub round_id: Option<String>,
}

pub async fn get_competition_widget(
    Path(space_id_raw): Path<String>,
    Query(query): Query<CompetitionWidgetQuery>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    CompetitionWidgetTemplate {
        routes: Default::default(),
        space_id: space_id_raw,
        show_detail: query.show_detail,
        show_rounds: query.show_rounds,
        selected_competition_id: query.competition_id.unwrap_or_default(),
        selected_season_id: query.season_id.unwrap_or_default(),
        selected_round_id: query.round_id.unwrap_or_default(),
    }
    .into_response()
}

// ── Endpoints JSON pour kreek-select ─────────────────────────────────────────

#[derive(Serialize)]
pub struct CompetitionJson {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct SeasonJson {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct RoundJson {
    pub id: String,
    pub name: String,
    pub dates: String,
    pub competition_id: String,
    pub season_id: String,
}

#[derive(Deserialize, Default)]
pub struct SeasonsJsonQuery {
    pub competition_id: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct RoundsJsonQuery {
    pub season_id: Option<String>,
}

pub async fn get_json_competitions(
    Path(space_id_raw): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Ok(space_id) = SpaceId::try_new(&space_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let competitions = match state
        .competitions
        .competition_repository
        .find_with_seasons(&space_id)
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("json_competitions find_with_seasons space={space_id_raw}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let result: Vec<CompetitionJson> = competitions
        .into_iter()
        .filter(|c| !c.seasons.is_empty())
        .map(|c| CompetitionJson {
            id: c.competition_id,
            name: c.competition_name,
        })
        .collect();

    Json(result).into_response()
}

pub async fn get_json_seasons(
    Path(space_id_raw): Path<String>,
    Query(query): Query<SeasonsJsonQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Ok(space_id) = SpaceId::try_new(&space_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let competition_id = query.competition_id.unwrap_or_default();

    let seasons: Vec<SeasonJson> = match state
        .competitions
        .competition_repository
        .find_with_seasons(&space_id)
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("json_seasons find_with_seasons space={space_id_raw}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    .into_iter()
    .find(|c| c.competition_id == competition_id)
    .map(|c| {
        c.seasons
            .into_iter()
            .rev()
            .map(|s| SeasonJson {
                id: s.season_id,
                name: s.season_name,
                status: s.status,
            })
            .collect()
    })
    .unwrap_or_default();

    Json(seasons).into_response()
}

pub async fn get_json_rounds(
    Path(space_id_raw): Path<String>,
    Query(query): Query<RoundsJsonQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let season_id = query.season_id.unwrap_or_default();

    let match_days = match state
        .competitions
        .match_day_repository
        .find_by_season(&season_id)
        .await
    {
        Ok(days) => days,
        Err(e) => {
            tracing::error!("json_rounds find_by_season {season_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let competition_id = match state
        .competitions
        .competition_repository
        .find_with_seasons(&SpaceId::try_new(&space_id_raw).unwrap_or_else(|_| SpaceId::new()))
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|c| c.seasons.iter().any(|s| s.season_id == season_id))
    {
        Some(c) => c.competition_id,
        None => String::new(),
    };

    let mut filtered: Vec<_> = match_days.into_iter().filter(|md| !md.is_rest()).collect();
    filtered.reverse();

    let rounds: Vec<RoundJson> = filtered
        .into_iter()
        .map(|md| {
            let dates = match (md.date_start, md.date_end) {
                (Some(s), Some(e)) => format!("{s} \u{2192} {e}"),
                (Some(s), None) => s.into_inner(),
                _ => String::new(),
            };
            RoundJson {
                id: md.id.to_string(),
                name: md.name.into_inner(),
                dates,
                competition_id: competition_id.clone(),
                season_id: season_id.clone(),
            }
        })
        .collect();

    Json(rounds).into_response()
}

// ── Panneau de détail (chargé au changement de saison) ───────────────────────

pub struct TierViewModel {
    pub name: String,
    pub budget: u32,
    pub start_xp: u32,
    pub rosters: Vec<String>,
}

pub struct RulesViewModel {
    pub win_pts: u32,
    pub draw_pts: u32,
    pub lose_pts: u32,
    pub tiers: Vec<TierViewModel>,
}

pub struct StructureViewModel {
    pub use_groups: bool,
    pub group_names: Vec<String>,
    pub use_schedule: bool,
    pub start_date: String,
    pub end_date: String,
}

/// Données de création copiées depuis la saison — sérialisées pour les contextes consommateurs.
/// Chaque contexte borné qui utilise la widget est libre de piocher ce dont il a besoin
/// dans ces champs cachés au moment de la soumission de son propre formulaire.
#[derive(Serialize)]
pub struct TierContext {
    pub name: String,
    pub budget: u32,
    pub start_xp: u32,
    pub rosters: Vec<String>,
}

#[derive(Serialize)]
pub struct CreationRulesContext {
    pub tiers: Vec<TierContext>,
}

#[derive(Template)]
#[template(path = "competition-widget-detail.html")]
pub struct CompetitionWidgetDetailTemplate {
    pub competition_id: String,
    pub competition_name: String,
    pub season_id: String,
    pub season_name: String,
    pub status: String,
    pub rules: Option<RulesViewModel>,
    pub structure: Option<StructureViewModel>,
    pub creation_rules_json: String,
}

impl IntoResponse for CompetitionWidgetDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_competition_widget_detail(
    Path((_space_id_raw, season_id_raw)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Ok(season_id) = SeasonId::try_new(&season_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let Some(full) = (match state
        .competitions
        .season_repository
        .find_full(&season_id)
        .await
    {
        Ok(opt) => opt,
        Err(e) => {
            tracing::error!("competition_widget_detail find_full season={season_id_raw}: {e}");
            None
        }
    }) else {
        return Html(
            r#"<div class="comp-detail-empty">Données indisponibles pour cette saison.</div>"#,
        )
        .into_response();
    };

    let creation_rules_json = full
        .rules
        .as_ref()
        .map(|r| {
            let ctx = CreationRulesContext {
                tiers: r
                    .tiers
                    .iter()
                    .map(|t| TierContext {
                        name: t.name.clone().into_inner(),
                        budget: t.budget.0,
                        start_xp: t.starting_xp.into_inner(),
                        rosters: t.rosters.clone(),
                    })
                    .collect(),
            };
            serde_json::to_string(&ctx).unwrap_or_default()
        })
        .unwrap_or_default();

    let rules = full.rules.map(|r| RulesViewModel {
        win_pts: r.ranking_rules.win_points.into_inner(),
        draw_pts: r.ranking_rules.draw_points.into_inner(),
        lose_pts: r.ranking_rules.lose_points.into_inner(),
        tiers: r
            .tiers
            .into_iter()
            .map(|t| TierViewModel {
                name: t.name.into_inner(),
                budget: t.budget.0,
                start_xp: t.starting_xp.into_inner(),
                rosters: t.rosters,
            })
            .collect(),
    });

    let structure = full.structure.map(|s| StructureViewModel {
        use_groups: s.ranking_group.use_ranking_groups(),
        group_names: s
            .ranking_group
            .groups()
            .iter()
            // `groups()` emprunte : le nom est cloné plutôt que déplacé, ce qui
            // est le prix — voulu — de ne plus laisser sortir l'agrégat.
            .map(|g| g.name.as_ref().to_string())
            .collect(),
        use_schedule: s.schedule.use_schedule.0,
        start_date: s.schedule.schedule_start_date.into_inner(),
        end_date: s.schedule.schedule_end_date.into_inner(),
    });

    CompetitionWidgetDetailTemplate {
        competition_id: full.competition_id,
        competition_name: full.competition_name,
        season_id: full.season_id,
        season_name: full.season_name,
        status: full.status,
        rules,
        structure,
        creation_rules_json,
    }
    .into_response()
}
