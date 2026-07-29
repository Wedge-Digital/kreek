use crate::app::auth::auth_backend::AuthSession;
use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::domain::space_repository_port::space_repository_port::SpaceSummary;
use crate::app::spaces::io::web::host_layout::ISpacesHostLayout;
use crate::app::spaces::routes::Routes;
use askama::Template;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

pub struct SidebarSpaceVm {
    pub name: String,
    pub logo: String,
    pub home_url: String,
    pub is_active: bool,
}

#[derive(Template)]
#[template(path = "widgets/spaces-sidebar.html")]
pub struct SpacesSidebar {
    pub routes: Routes,
    pub content_target: String,
    pub spaces: Vec<SidebarSpaceVm>,
}

impl IntoResponse for SpacesSidebar {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// `HX-Current-URL` est une URL complète : `http://host/app/{space_id}/home`.
/// On en retire le schéma, l'hôte, la query et le fragment pour obtenir un
/// chemin comparable à celui que l'hôte construit pour l'accueil d'un espace.
fn current_path(current_url: &str) -> Option<&str> {
    let path = match current_url.find("://") {
        Some(i) => current_url[i + 3..]
            .find('/')
            .map(|j| &current_url[i + 3 + j..])?,
        None => current_url,
    };
    path.split('?').next()?.split('#').next()
}

/// L'espace actif est celui dont la page d'accueil — fournie par l'hôte — est
/// la page courante. Le BC n'a donc pas à connaître la forme de cette route
/// pour la reconnaître.
fn to_vm(
    space: SpaceSummary,
    active_path: Option<&str>,
    host_layout: &dyn ISpacesHostLayout,
) -> SidebarSpaceVm {
    let home_url = host_layout.space_home(&space.id);
    SidebarSpaceVm {
        is_active: active_path == Some(home_url.as_str()),
        name: space.name,
        logo: space.logo.thumbnail(100, 100),
        home_url,
    }
}

pub async fn get_spaces_sidebar(
    auth_session: AuthSession,
    State(ctx): State<SpacesContext>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let spaces = ctx
        .space_repository
        .find_by_coach_id(&user.id)
        .await
        .unwrap_or_default();

    let active_path = headers
        .get("hx-current-url")
        .and_then(|v| v.to_str().ok())
        .and_then(current_path);

    SpacesSidebar {
        routes: Routes,
        content_target: ctx.host_layout.content_target(),
        spaces: spaces
            .into_iter()
            .map(|s| to_vm(s, active_path, ctx.host_layout.as_ref()))
            .collect(),
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::identity::ids::CloudinaryImage;
    use crate::app::spaces::io::web::host_layout::UploadField;

    struct FakeHostLayout;

    impl ISpacesHostLayout for FakeHostLayout {
        fn wrap_page(&self, _content: String) -> Response {
            unimplemented!("le sidebar ne rend jamais de page complète")
        }
        fn content_target(&self) -> String {
            "#app-content".to_string()
        }
        fn space_home(&self, space_id: &str) -> String {
            format!("/app/{space_id}/home")
        }
        fn unauthenticated_redirect(&self) -> String {
            "/auth".to_string()
        }
        fn upload_widget(&self, _field: UploadField<'_>) -> String {
            unimplemented!("le sidebar n'affiche aucun champ d'upload")
        }
    }

    fn space(id: &str) -> SpaceSummary {
        SpaceSummary {
            id: id.to_string(),
            name: format!("Espace {id}"),
            logo: CloudinaryImage::try_new("https://res.cloudinary.com/demo/logo.png").unwrap(),
        }
    }

    #[test]
    fn current_path_retire_le_schema_et_l_hote() {
        assert_eq!(
            current_path("http://localhost:3000/app/42/home"),
            Some("/app/42/home")
        );
    }

    #[test]
    fn current_path_retire_la_query_et_le_fragment() {
        assert_eq!(
            current_path("http://localhost/app/42/home?tab=1#bas"),
            Some("/app/42/home")
        );
    }

    #[test]
    fn current_path_accepte_un_chemin_nu() {
        assert_eq!(current_path("/app/42/home"), Some("/app/42/home"));
    }

    #[test]
    fn l_espace_dont_l_accueil_est_la_page_courante_est_actif() {
        let vm = to_vm(space("42"), Some("/app/42/home"), &FakeHostLayout);
        assert!(vm.is_active);
        assert_eq!(vm.home_url, "/app/42/home");
    }

    #[test]
    fn les_autres_espaces_ne_sont_pas_actifs() {
        let vm = to_vm(space("7"), Some("/app/42/home"), &FakeHostLayout);
        assert!(!vm.is_active);
    }

    #[test]
    fn aucun_espace_n_est_actif_hors_d_une_page_d_accueil() {
        let vm = to_vm(space("42"), Some("/app/space/all"), &FakeHostLayout);
        assert!(!vm.is_active);
    }
}
