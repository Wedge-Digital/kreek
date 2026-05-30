use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::competitions::routes::Routes as CompetitionRoutes;
use crate::app::shared_kernel::common_types::{SeasonId, SpaceId};
use crate::state::AppState;

// ── Widget principal : sélecteur compétition ─────────────────────────────────

pub struct CompetitionItem {
    pub competition_id:   String,
    pub competition_name: String,
}

#[derive(Template)]
#[template(path = "competition-widget.html")]
pub struct CompetitionWidgetTemplate {
    pub routes:       CompetitionRoutes,
    pub space_id:     String,
    pub competitions: Vec<CompetitionItem>,
    pub show_detail:  bool,
}

impl IntoResponse for CompetitionWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct CompetitionWidgetQuery {
    #[serde(default)]
    pub show_detail: bool,
}

pub async fn get_competition_widget(
    Path(space_id_raw): Path<String>,
    Query(query):       Query<CompetitionWidgetQuery>,
    State(state):       State<AppState>,
) -> impl IntoResponse {
    let Ok(space_id) = SpaceId::try_new(&space_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let competitions = match state.competitions.competition_repository
        .find_with_seasons(&space_id)
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("competition_widget find_with_seasons space={space_id_raw}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    .into_iter()
    .map(|c| CompetitionItem {
        competition_id:   c.competition_id,
        competition_name: c.competition_name,
    })
    .collect();

    CompetitionWidgetTemplate {
        routes:      Default::default(),
        space_id:    space_id_raw,
        competitions,
        show_detail: query.show_detail,
    }.into_response()
}

// ── Fragment saisons (chargé au changement de compétition) ───────────────────

pub struct SeasonItem {
    pub season_id:   String,
    pub season_name: String,
    pub status:      String,
}

#[derive(Template)]
#[template(path = "competition-widget-seasons.html")]
pub struct SeasonSelectorTemplate {
    pub routes:      CompetitionRoutes,
    pub space_id:    String,
    pub seasons:     Vec<SeasonItem>,
    pub show_detail: bool,
}

impl IntoResponse for SeasonSelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct SeasonsQuery {
    pub competition_id: Option<String>,
    #[serde(default)]
    pub show_detail:    bool,
}

pub async fn get_competition_widget_seasons(
    Path(space_id_raw): Path<String>,
    Query(query):       Query<SeasonsQuery>,
    State(state):       State<AppState>,
) -> impl IntoResponse {
    let Ok(space_id) = SpaceId::try_new(&space_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let competition_id = query.competition_id.unwrap_or_default();

    let seasons = match state.competitions.competition_repository
        .find_with_seasons(&space_id)
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("competition_widget_seasons find_with_seasons space={space_id_raw}: {e}");
            vec![]
        }
    }
    .into_iter()
    .find(|c| c.competition_id == competition_id)
    .map(|c| c.seasons.into_iter().map(|s| SeasonItem {
        season_id:   s.season_id,
        season_name: s.season_name,
        status:      s.status,
    }).collect())
    .unwrap_or_default();

    SeasonSelectorTemplate {
        routes:      Default::default(),
        space_id:    space_id_raw,
        seasons,
        show_detail: query.show_detail,
    }.into_response()
}

// ── Panneau de détail (chargé au changement de saison) ───────────────────────

pub struct TierViewModel {
    pub name:      String,
    pub budget:    u32,
    pub start_xp:  u32,
    pub rosters:   Vec<String>,
}

pub struct RulesViewModel {
    pub win_pts:  u32,
    pub draw_pts: u32,
    pub lose_pts: u32,
    pub tiers:    Vec<TierViewModel>,
}

pub struct StructureViewModel {
    pub use_groups:   bool,
    pub group_names:  Vec<String>,
    pub use_playoffs: bool,
    pub use_schedule: bool,
    pub start_date:   String,
    pub end_date:     String,
}

#[derive(Template)]
#[template(path = "competition-widget-detail.html")]
pub struct CompetitionWidgetDetailTemplate {
    pub competition_id:   String,
    pub competition_name: String,
    pub season_id:        String,
    pub season_name:      String,
    pub status:           String,
    pub rules:            Option<RulesViewModel>,
    pub structure:        Option<StructureViewModel>,
}

impl IntoResponse for CompetitionWidgetDetailTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_competition_widget_detail(
    Path((_space_id_raw, season_id_raw)): Path<(String, String)>,
    State(state):                          State<AppState>,
) -> impl IntoResponse {
    let Ok(season_id) = SeasonId::try_new(&season_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let Some(full) = (match state.competitions.season_repository
        .find_full(&season_id)
        .await
    {
        Ok(opt) => opt,
        Err(e) => {
            tracing::error!("competition_widget_detail find_full season={season_id_raw}: {e}");
            None
        }
    }) else {
        return Html(r#"<div class="comp-detail-empty">Données indisponibles pour cette saison.</div>"#).into_response();
    };

    let rules = full.rules.map(|r| RulesViewModel {
        win_pts:  r.ranking_rules.win_points,
        draw_pts: r.ranking_rules.draw_points,
        lose_pts: r.ranking_rules.lose_points,
        tiers:    r.tiers.into_iter().map(|t| TierViewModel {
            name:     t.name,
            budget:   t.budget,
            start_xp: t.starting_xp,
            rosters:  t.rosters,
        }).collect(),
    });

    let structure = full.structure.map(|s| StructureViewModel {
        use_groups:   s.ranking_group.use_ranking_groups,
        group_names:  s.ranking_group.ranking_groups.into_iter().map(|g| g.name).collect(),
        use_playoffs: s.play_offs_phase.use_playoffs_phase,
        use_schedule: s.schedule.use_schedule,
        start_date:   s.schedule.schedule_start_date,
        end_date:     s.schedule.schedule_end_date,
    });

    CompetitionWidgetDetailTemplate {
        competition_id:   full.competition_id,
        competition_name: full.competition_name,
        season_id:        full.season_id,
        season_name:      full.season_name,
        status:           full.status,
        rules,
        structure,
    }.into_response()
}