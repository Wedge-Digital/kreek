use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

/// Un champ d'upload d'image que l'hôte rend pour le compte du BC.
pub struct UploadField<'a> {
    pub field_id: &'a str,
    pub initial_value: &'a str,
    pub folder: &'a str,
    pub label: &'a str,
    pub error: Option<&'a str>,
}

/// Ce que le BC `spaces` attend de l'application qui l'héberge.
///
/// Askama résout `extends` statiquement : un BC ne peut pas recevoir son
/// layout en paramètre de template. Ses pages ne produisent donc que des
/// fragments, et c'est l'hôte — au travers de ce trait — qui les enveloppe
/// dans son propre cadre et qui fournit les destinations dont il est le seul
/// propriétaire.
pub trait ISpacesHostLayout: Send + Sync {
    /// Enveloppe un fragment de page dans le document complet de l'hôte.
    fn wrap_page(&self, content: String) -> Response;

    /// Sélecteur CSS de la zone de contenu de l'hôte, cible des `hx-target`
    /// des liens de navigation du BC.
    fn content_target(&self) -> String;

    /// Accueil d'un espace. La page est fournie par l'hôte, pas par `spaces`.
    fn space_home(&self, space_id: &str) -> String;

    /// Où renvoyer un visiteur non authentifié.
    fn unauthenticated_redirect(&self) -> String;

    /// Rend un champ d'upload d'image. Le composant, son service de stockage
    /// et le compte associé appartiennent à l'hôte — le BC décrit seulement le
    /// champ qu'il veut voir.
    fn upload_widget(&self, field: UploadField<'_>) -> String;
}

/// Rend une page du BC : fragment nu pour une requête HTMX, document complet
/// de l'hôte pour un chargement classique (accès direct, F5, redirection).
pub fn render_page<T: Template>(
    page: T,
    headers: &HeaderMap,
    host_layout: &dyn ISpacesHostLayout,
) -> Response {
    match page.render() {
        Ok(content) if headers.contains_key("hx-request") => Html(content).into_response(),
        Ok(content) => host_layout.wrap_page(content),
        Err(e) => {
            tracing::error!("render failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
