use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::common_types::Entity;
use crate::app::team_creation::routes::Routes as TeamRoutes;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;

pub struct TeamCardVm {
    pub id:           String,
    pub name:         String,
    pub logo:         Option<String>,
    pub roster:       String,
    pub tv:           u32,
    pub status:       String,
    pub status_label: String,
    pub link:         String,
}

#[derive(Template)]
#[template(path = "my-teams.html")]
pub struct MyTeamsTemplate {
    pub web_routes:  WebRoutes,
    pub team_routes: TeamRoutes,
    pub space_id:    String,
    pub teams:       Vec<TeamCardVm>,
}

impl IntoResponse for MyTeamsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn my_teams(
    auth_session:       AuthSession,
    Path(space_id_raw): Path<String>,
    State(state):       State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let teams = match state.team_creation.team_repository
        .find_by_coach_and_space(&user.id.to_string(), &space_id_raw)
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("my_teams find_by_coach_and_space: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let submitted_ids = state.team_creation.roster_repository
        .find_submitted_ids_for_space(&space_id_raw)
        .await
        .unwrap_or_default();

    let routes: crate::app::team_creation::routes::Routes = Default::default();
    let team_vms = teams.into_iter().map(|t| {
        let id = t.get_id().to_string();
        let submitted = submitted_ids.contains(&id);
        let link = if submitted {
            routes.team_detail(&space_id_raw, &id)
        } else {
            routes.team_build(&space_id_raw, &id)
        };
        TeamCardVm {
            id,
            name:         t.base_infos().name().clone().into_inner(),
            logo:         t.base_infos().logo_url().map(|u| u.as_ref().to_string()),
            roster:       String::new(),
            tv:           0,
            status:       if submitted { "active".into() } else { "draft".into() },
            status_label: if submitted { "Inscrite".into() } else { "Brouillon".into() },
            link,
        }
    }).collect();

    MyTeamsTemplate {
        web_routes:  Default::default(),
        team_routes: Default::default(),
        space_id:    space_id_raw,
        teams:       team_vms,
    }.into_response()
}