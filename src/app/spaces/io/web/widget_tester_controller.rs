use crate::app::spaces::routes::Routes;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::shared_kernel::identity::space_definition::SpaceDefinition;
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::spaces::context::SpacesContext;


#[derive(Template)]
#[template(path = "pages/spaces-widget-tester-page.html")]
pub struct SpacesWidgetPageTesterTemplate {
    pub routes: Routes,
    pub spaces: Vec<SpaceDefinition>,
}

pub async fn get_space_widget_tester(
    State(ctx): State<SpacesContext>,
) -> impl IntoResponse {
    let spaces = ctx.space_repository.find_all()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| SpaceDefinition {
            id: SpaceId::try_new(&s.id).expect(""),
            name: SpaceName::try_new(&s.name).expect("") })
        .collect();

    SpacesWidgetPageTesterTemplate {
        routes: Routes,
        spaces,
    }.into_response()
}

impl IntoResponse for SpacesWidgetPageTesterTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}