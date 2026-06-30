use crate::app::match_report::ports::{PositionCountDto, RosterPositionDto};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct PositionCardVm {
    pub uid:           String,
    pub name:          String,
    pub base_cost:     u32,
    pub price_base:    u32,
    pub price_lvl1:    u32,
    pub count_in_team: u8,
    pub max_qty:       u8,
    pub disabled:      bool,
}

#[derive(Template)]
#[template(path = "match_report/widgets/mercenary-selector-widget.html")]
pub struct MercenarySelectorTemplate {
    pub positions: Vec<PositionCardVm>,
}

impl IntoResponse for MercenarySelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_mercenary_selector(
    Path((_space_id, _mr_id, team_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let positions = state.match_report.team_data.find_roster_positions(&team_id).await;
    let counts = state.match_report.player_data.find_player_counts_by_position(&team_id).await;
    let vms = build_position_grid(positions, counts);
    MercenarySelectorTemplate { positions: vms }.into_response()
}

fn build_position_grid(
    positions: Vec<RosterPositionDto>,
    counts: Vec<PositionCountDto>,
) -> Vec<PositionCardVm> {
    positions
        .into_iter()
        .map(|p| {
            let count_in_team = counts
                .iter()
                .find(|c| c.position_uid == p.position_uid)
                .map(|c| c.count)
                .unwrap_or(0);
            PositionCardVm {
                price_base:    p.base_cost + 30,
                price_lvl1:    p.base_cost + 80,
                disabled:      count_in_team >= p.max_qty,
                uid:           p.position_uid,
                name:          p.position_name,
                base_cost:     p.base_cost,
                count_in_team,
                max_qty:       p.max_qty,
            }
        })
        .collect()
}
