use crate::app::auth::auth_backend::AuthSession;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

const BYPASS_AUTH_LEGACY_USER_ID: i32 = 1;

/// En-tête choisissant l'identité connectée. Absent ou non reconnu → `DevCoach`,
/// c'est-à-dire le comportement historique, inchangé.
///
/// Cet en-tête n'a d'effet que si `bypass_auth` est actif — un profil de
/// développement. En production le middleware ne connecte personne, et l'en-tête
/// n'ouvre donc aucune porte.
const BYPASS_AUTH_PROFILE_HEADER: &str = "x-bypass-auth-profile";
const BYPASS_AUTH_SIMPLE_PROFILE: &str = "simple";

fn demande_profil_simple(request: &Request<Body>) -> bool {
    request
        .headers()
        .get(BYPASS_AUTH_PROFILE_HEADER)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case(BYPASS_AUTH_SIMPLE_PROFILE))
}

pub async fn bypass_auth_middleware(
    State(state): State<AppState>,
    mut auth_session: AuthSession,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();

    tracing::debug!(%path, has_user = auth_session.user.is_some(), "bypass_auth: entrée");

    // Ne s'active que sur une session vide : une identité déjà connectée n'est
    // jamais remplacée. Un test qui veut l'autre identité doit donc partir d'un
    // contexte navigateur neuf, sans cookie de session.
    if state.bypass_auth && auth_session.user.is_none() {
        // Le membre simple est repéré par son nom, pas par un `legacy_id` : cet
        // espace d'identifiants appartient au système legacy, dont l'import
        // occupe déjà les premières valeurs.
        let simple = demande_profil_simple(&request);
        tracing::debug!(%path, simple, "bypass_auth: login automatique");
        let recherche = match simple {
            true => {
                state
                    .auth
                    .user_repository
                    .find_by_coach_name(crate::cli::seed_e2e::SIMPLE_COACH_NAME)
                    .await
            }
            false => {
                state
                    .auth
                    .user_repository
                    .find_by_legacy_id(BYPASS_AUTH_LEGACY_USER_ID)
                    .await
            }
        };
        match recherche {
            Ok(Some(user)) => {
                if auth_session.login(&user).await.is_ok() {
                    tracing::debug!(%path, "bypass_auth: login OK");
                    request.extensions_mut().insert(auth_session);
                } else {
                    tracing::warn!(%path, "bypass_auth: login échoué");
                }
            }
            Ok(None) => {
                tracing::warn!(%path, simple, "bypass_auth: utilisateur introuvable");
            }
            Err(e) => {
                tracing::warn!(%path, error = %e, "bypass_auth: erreur lors de la recherche utilisateur");
            }
        }
    }

    tracing::debug!(%path, "bypass_auth: passage au handler suivant");
    let response = next.run(request).await;
    tracing::debug!(%path, status = %response.status(), "bypass_auth: réponse reçue");
    response
}
