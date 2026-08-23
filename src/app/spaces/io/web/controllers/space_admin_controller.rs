use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use crate::app::spaces::io::web::host_layout::render_page;
use crate::app::spaces::routes::Routes;
use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

#[derive(Template)]
#[template(path = "space-admin.html")]
pub struct SpaceAdminPageTemplate {
    pub routes: Routes,
    pub space_id: String,
    pub space_name: String,
    pub content_target: String,
}

/// La page d'administration d'un espace.
///
/// Assemblage pur : bannière, barre d'onglets, zone de contenu. Aucun calcul de
/// VM, aucune logique — le patron « page d'assemblage à widgets ».
///
/// La garde est ici **et** sur chacun des endpoints de widget : un widget
/// n'hérite d'aucune protection de sa page hôte, son endpoint étant directement
/// atteignable.
pub async fn space_admin_controller(
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
    headers: HeaderMap,
) -> Response {
    if !perms.is_admin() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(Some(space)) = ctx.space_repository.find_by_id(&perms.space_id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let page = SpaceAdminPageTemplate {
        routes: Routes::default(),
        space_id: perms.space_id.to_string(),
        space_name: space.name().to_string(),
        content_target: ctx.host_layout.content_target(),
    };
    render_page(page, &headers, ctx.host_layout.as_ref())
}
