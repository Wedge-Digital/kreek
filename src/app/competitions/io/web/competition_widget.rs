use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::competitions::routes::Routes as CompetitionRoutes;
use crate::app::shared_kernel::common_types::{SeasonId, SpaceId};
use crate::state::AppState;

// ── Widget principal (liste des compétitions + sélecteur) ───────────────────

pub struct SeasonWidgetItem {
    pub season_id:   String,
    pub season_name: String,
    pub status:      String,
    pub selected:    bool,
}

pub struct CompetitionWidgetGroup {
    pub competition_id:   String,
    pub competition_name: String,
    pub seasons:          Vec<SeasonWidgetItem>,
}

#[derive(Template)]
#[template(path = "competition-widget.html")]
pub struct CompetitionWidgetTemplate {
    pub routes:   CompetitionRoutes,
    pub space_id: String,
    pub groups:   Vec<CompetitionWidgetGroup>,
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
    pub selected: Option<String>,
}

pub async fn get_competition_widget(
    Path(space_id_raw): Path<String>,
    Query(query):       Query<CompetitionWidgetQuery>,
    State(state):       State<AppState>,
) -> impl IntoResponse {
    let Ok(space_id) = SpaceId::try_new(&space_id_raw) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let selected = query.selected.clone().unwrap_or_default();

    let competitions = state.competitions.competition_repository
        .find_with_seasons(&space_id)
        .await
        .unwrap_or_default();

    let groups = competitions.into_iter().map(|c| CompetitionWidgetGroup {
        competition_id:   c.competition_id,
        competition_name: c.competition_name,
        seasons: c.seasons.into_iter().map(|s| SeasonWidgetItem {
            selected:    s.season_id == selected,
            season_id:   s.season_id,
            season_name: s.season_name,
            status:      s.status,
        }).collect(),
    }).collect();

    CompetitionWidgetTemplate {
        routes:   Default::default(),
        space_id: space_id_raw,
        groups,
    }.into_response()
}

// ── Panneau de détail (chargé au changement de sélection) ──────────────────

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

    let Some(full) = state.competitions.season_repository
        .find_full(&season_id)
        .await
        .unwrap_or(None)
    else {
        return StatusCode::NOT_FOUND.into_response();
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