//! Le journal de requêtes de l'application — un seul, actif dans tous les
//! builds.
//!
//! Il a longtemps été branché à l'intérieur du bloc `#[cfg(debug_assertions)]`
//! de `build_router`, dont il avait hérité la condition en voisinant
//! `tower-livereload`. En production il n'existait donc pas, et le
//! `TraceLayer` censé le remplacer émettait sur `tower_http::trace`, cible que
//! le filtre par défaut `kreek=debug` n'active pas : chaque ligne du journal y
//! était un `error!` isolé, sans méthode, sans chemin, sans statut (carte 344).

use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn request_log(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    // Aucun champ de corrélation ici, et c'est délibéré : l'identifiant de
    // thread qui tenait ce rôle ne corrèle rien en async — une tâche migre
    // d'un thread à l'autre entre deux `await`, et des requêtes concurrentes
    // partagent le même thread. Un champ qui ment est pire qu'un champ absent.
    // Le vrai identifiant de requête arrive avec la carte 345.
    tracing::info!(%method, %path, "→ requête reçue");

    let response = next.run(request).await;

    tracing::info!(%method, %path, status = %response.status(), "← réponse envoyée");
    response
}
