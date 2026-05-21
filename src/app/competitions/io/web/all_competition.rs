use askama::Template;
use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use crate::app::competitions::routes::Routes;
use crate::web::app_layout::AppLayout;

pub struct CompetitionSummary {
    pub id:   String,
    pub name: String,
}

#[derive(Template)]
#[template(path = "all-competitions.html")]
pub struct AllCompetitionTemplate {
    pub competition_routes: Routes,
    pub space_id:           String,
    pub competitions:       Vec<CompetitionSummary>,
}

impl Default for AllCompetitionTemplate {
    fn default() -> Self {
        Self {
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

pub async fn get_all_competition(Path(space_id): Path<String>, headers: HeaderMap) -> impl IntoResponse {
    let tmpl = AllCompetitionTemplate {
        space_id,
        competitions: vec![],   // remplacer par un appel repository quand le modèle existera
        ..Default::default()
    };
    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}