use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::references::io::web::pickers::{build_roster_items_with_tiers, RosterPickerItemWithTier};
use crate::app::shared_kernel::common_types::EntityId;
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;

#[derive(Template)]
#[template(path = "build-team.html")]
pub struct BuildTeamTemplate {
    pub web_routes:  WebRoutes,
    pub team_routes: TeamCreationRoutes,
    pub space_id:    String,
    pub team_id:     String,
    pub rosters:     Vec<RosterPickerItemWithTier>,
}

impl IntoResponse for BuildTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn build_team(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let draft = match state.team_creation.team_repository.find_by_id(&team_id_val).await {
        Ok(Some(t)) => t,
        Ok(None)    => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("build_team find_by_id {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let rosters = build_roster_items_with_tiers(
        state.references.repository.as_ref(),
        draft.creation_rules(),
    );

    BuildTeamTemplate {
        web_routes:  Default::default(),
        team_routes: Default::default(),
        space_id,
        team_id,
        rosters,
    }.into_response()
}