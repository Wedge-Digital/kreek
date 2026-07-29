use crate::app::auth::routes::path as auth_path;
use crate::app::news::routes::Routes as NewsRoutes;
use crate::app::routes::AppRoutes;
use crate::app::spaces::io::web::host_layout::{ISpacesHostLayout, UploadField};
use crate::web::app_shell::{AppShell, CONTENT_TARGET};
use crate::web::upload_widget::render_upload_widget;
use axum::response::{IntoResponse, Response};

/// Le cadre que kreek fournit au BC `spaces` : son layout applicatif, et les
/// deux destinations dont il est propriétaire — l'accueil d'un espace, qui
/// appartient à `news`, et l'écran de connexion, qui appartient à `auth`.
///
/// Un autre hôte fournirait sa propre implémentation ; c'est le seul point du
/// projet qui relie `spaces` au reste de kreek.
pub struct KreekSpacesLayout;

impl ISpacesHostLayout for KreekSpacesLayout {
    fn wrap_page(&self, content: String) -> Response {
        AppShell {
            app_routes: AppRoutes::default(),
            content,
        }
        .into_response()
    }

    fn content_target(&self) -> String {
        CONTENT_TARGET.to_string()
    }

    fn space_home(&self, space_id: &str) -> String {
        NewsRoutes::default().space_home(space_id)
    }

    fn unauthenticated_redirect(&self) -> String {
        auth_path::AUTH_LAYOUT.to_string()
    }

    fn upload_widget(&self, field: UploadField<'_>) -> String {
        render_upload_widget(
            field.field_id,
            field.initial_value,
            field.folder,
            field.label,
            field.error,
        )
    }
}
