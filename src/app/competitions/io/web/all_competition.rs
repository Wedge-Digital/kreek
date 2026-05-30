use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::competitions::domain::competition_repository_port::CompetitionSummary;
use crate::app::competitions::routes::Routes;
use crate::app::shared_kernel::common_types::SpaceId;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;

pub struct CompetitionCardViewModel {
    pub id:           String,
    pub name:         String,
    pub logo:         String,
    pub status:       String,
    pub season_count: i64,
    pub url:          String,
}

fn card_url(routes: &Routes, space_id: &str, c: &CompetitionSummary) -> String {
    let status = c.status.as_deref().unwrap_or("draft");
    match c.season_id.as_deref() {
        Some(sid) => match status {
            "draft"                  => routes.new_competition_rules(space_id, &c.id, sid),
            "rules_selected"         => routes.new_competition_structure(space_id, &c.id, sid),
            "structure_selected"     => routes.new_competition_invitations(space_id, &c.id, sid),
            "invitations_configured" => routes.new_competition_validation(space_id, &c.id, sid),
            _                        => routes.competition_detail(space_id, &c.id, sid),
        },
        None => routes.new_competition_info(space_id, &c.id),
    }
}

#[derive(Template)]
#[template(path = "all-competitions.html")]
pub struct AllCompetitionTemplate {
    pub web_routes:         WebRoutes,
    pub competition_routes: Routes,
    pub space_id:           String,
    pub competitions:       Vec<CompetitionCardViewModel>,
}

impl Default for AllCompetitionTemplate {
    fn default() -> Self {
        Self {
            web_routes:         WebRoutes,
            competition_routes: Routes,
            space_id:           String::new(),
            competitions:       Vec::new(),
        }
    }
}

impl IntoResponse for AllCompetitionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_all_competition(
    Path(space_id): Path<String>,
    State(state):   State<AppState>,
) -> impl IntoResponse {
    let summaries = match SpaceId::try_new(&space_id) {
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        Ok(id) => match state.competitions.competition_repository.find_by_space_id(&id).await {
            Ok(list) => list,
            Err(_)   => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    };

    let routes = Routes;
    let competitions = summaries.iter().map(|c| {
        let url    = card_url(&routes, &space_id, c);
        let status = c.status.clone().unwrap_or_else(|| "draft".to_string());
        CompetitionCardViewModel {
            id:           c.id.clone(),
            name:         c.name.clone(),
            logo:         c.logo.clone(),
            status,
            season_count: c.season_count,
            url,
        }
    }).collect();

    AllCompetitionTemplate { space_id, competitions, ..Default::default() }.into_response()
}
