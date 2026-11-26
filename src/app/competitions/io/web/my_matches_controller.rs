//! La page « Mes matchs » (carte 677).
//!
//! **Page hôte sans logique** : elle ne lit rien, et compose le widget
//! `my_matches_widget`, qui reçoit l'espace dans son URL.

use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::SpaceId;
use askama::Template;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "my-matches.html")]
pub struct MyMatchesPageTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
}

impl IntoResponse for MyMatchesPageTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("my_matches page render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_my_matches_page(Path(space_id): Path<String>) -> Response {
    if SpaceId::try_new(&space_id).is_err() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    MyMatchesPageTemplate {
        app_routes: AppRoutes::default(),
        space_id,
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPACE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    #[test]
    fn the_page_loads_the_widget_of_its_space() {
        let html = MyMatchesPageTemplate {
            app_routes: AppRoutes::default(),
            space_id: SPACE.into(),
        }
        .render()
        .unwrap();
        let widget = AppRoutes::default().competitions.my_matches_widget(SPACE);
        assert!(html.contains("Mes matchs"));
        assert!(html.contains(&format!("hx-get=\"{widget}\"")), "{html}");
    }

    #[tokio::test]
    async fn an_unreadable_space_is_refused() {
        let response = get_my_matches_page(Path("pas-un-espace".into())).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
