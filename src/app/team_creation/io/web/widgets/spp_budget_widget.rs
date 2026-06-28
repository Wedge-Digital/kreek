use crate::app::shared_kernel::common_types::EntityId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct SppBudgetVm {
    pub spp_pool: u8,
    pub spp_spent: u8,
    pub spp_total: u8,
    pub spp_pct: u8,
}

#[derive(Template)]
#[template(path = "widgets/spp-budget-widget.html")]
pub struct SppBudgetWidgetTemplate {
    pub budget: Option<SppBudgetVm>,
}

impl IntoResponse for SppBudgetWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn spp_budget_widget(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let budget = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(team)) => {
            let spp_spent: u8 = team
                .hired_players()
                .iter()
                .flat_map(|p| p.acquired_skills.iter())
                .map(|a| a.spp_cost.into_inner())
                .sum();
            let spp_total = team.spp_pool.0 + spp_spent;
            let spp_pct = if spp_total > 0 {
                (spp_spent as u16 * 100 / spp_total as u16) as u8
            } else {
                0
            };
            Some(SppBudgetVm {
                spp_pool: team.spp_pool.0,
                spp_spent,
                spp_total,
                spp_pct,
            })
        }
        _ => None,
    };

    SppBudgetWidgetTemplate { budget }.into_response()
}
