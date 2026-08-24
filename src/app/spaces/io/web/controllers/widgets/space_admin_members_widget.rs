use crate::app::auth::auth_backend::AuthSession;
use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::io::web::builders::{build_member_rows, MemberRowVm};
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use crate::app::spaces::routes::Routes;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "widgets/space-admin-members.html")]
pub struct SpaceAdminMembersTemplate {
    pub routes: Routes,
    pub space_id: String,
    pub membres: Vec<MemberRowVm>,
}

impl IntoResponse for SpaceAdminMembersTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("space_admin_members_widget: rendu impossible: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// La liste des membres de l'espace.
///
/// La garde est répétée ici : un widget n'hérite d'aucune protection de sa page
/// hôte, son endpoint étant directement atteignable.
pub async fn space_admin_members_widget(
    auth_session: AuthSession,
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
) -> Response {
    if !perms.is_admin() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(moi) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(lignes) = ctx
        .space_repository
        .list_members_with_profile(&perms.space_id)
        .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    SpaceAdminMembersTemplate {
        routes: Routes::default(),
        space_id: perms.space_id.to_string(),
        membres: build_member_rows(lignes, &moi.id),
    }
    .into_response()
}
