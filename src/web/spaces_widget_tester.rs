use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::shared_kernel::identity::space_definition::SpaceDefinition;
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::spaces::context::SpacesContext;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Outil de développement de l'hôte : il compose les widgets exposés par
/// `spaces` pour les essayer isolément. Il ne fait pas partie du BC extrait —
/// c'est pourquoi il vit ici et peut, lui, passer par `AppRoutes`.
#[derive(Template)]
#[template(path = "spaces-widget-tester-page.html")]
pub struct SpacesWidgetPageTesterTemplate {
    pub app_routes: AppRoutes,
    pub spaces: Vec<SpaceDefinition>,
}

pub async fn get_space_widget_tester(State(ctx): State<SpacesContext>) -> impl IntoResponse {
    let spaces = ctx
        .space_repository
        .find_all()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| SpaceDefinition {
            id: SpaceId::try_new(&s.id).expect(""),
            name: SpaceName::try_new(&s.name).expect(""),
        })
        .collect();

    SpacesWidgetPageTesterTemplate {
        app_routes: AppRoutes::default(),
        spaces,
    }
    .into_response()
}

impl IntoResponse for SpacesWidgetPageTesterTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
