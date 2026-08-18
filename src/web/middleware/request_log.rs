//! Le journal de requêtes de l'application — un seul, actif dans tous les
//! builds.
//!
//! Il a longtemps été branché à l'intérieur du bloc `#[cfg(debug_assertions)]`
//! de `build_router`, dont il avait hérité la condition en voisinant
//! `tower-livereload`. En production il n'existait donc pas, et le
//! `TraceLayer` censé le remplacer émettait sur `tower_http::trace`, cible que
//! le filtre par défaut `kreek=debug` n'active pas : chaque ligne du journal y
//! était un `error!` isolé, sans méthode, sans chemin, sans statut (carte 344).
//!
//! # Le span, et ce qu'il rend possible
//!
//! Ouvrir un span par requête fait que **toute** ligne émise en dessous en
//! hérite — les 198 `tracing::error!` du projet portent désormais le `rid` de
//! la requête qui les a provoqués, sans qu'aucun n'ait été modifié. C'est ce
//! qui transforme des cris isolés en récits (carte 345).
//!
//! Le `rid` est repris dans l'en-tête `x-request-id` de la réponse : c'est le
//! seul moyen de partir d'un symptôme constaté par un coach — l'identifiant lu
//! dans l'onglet réseau de son navigateur — pour retrouver la requête dans
//! `docker logs`.
//!
//! **On ne reprend jamais un `x-request-id` entrant.** L'honorer permettrait à
//! n'importe qui d'injecter du texte arbitraire dans les journaux, et aucun
//! proxy ne le pose aujourd'hui.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::identity::sulid::SUlid;
use axum::body::Body;
use axum::http::{HeaderValue, Request};
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;
use tracing::Instrument;

/// `AuthSession` en premier paramètre impose que cette couche soit posée **à
/// l'intérieur** d'`AuthManagerLayer` : la session n'existe pas avant. Ça ne
/// coûte rien à ce qu'apportait la carte 344 — `AuthManagerLayer` ne rejette
/// personne, elle charge. Les refus viennent de `require_auth` et de
/// `space_scope`, posés en `route_layer` plus profond, donc toujours
/// enveloppés par ce journal.
pub async fn request_log(
    auth_session: AuthSession,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let rid = SUlid::new().to_string();
    let coach = nom_du_coach(&auth_session);

    let span = tracing::info_span!("req", %rid, %method, %path, %coach);
    journaliser(request, next, rid).instrument(span).await
}

/// La durée est mesurée ici plutôt que laissée à `FmtSpan::CLOSE` : le
/// réglage global émettrait une paire de lignes par span — quatre par requête
/// — et sa ligne de fermeture porte le temps sans le statut. Ici, statut et
/// durée tiennent sur la même ligne, ce que `grep` lit d'un coup.
async fn journaliser(request: Request<Body>, next: Next, rid: String) -> Response {
    tracing::info!("→ requête reçue");
    let debut = Instant::now();

    let mut response = next.run(request).await;

    tracing::info!(
        status = %response.status(),
        duree_ms = debut.elapsed().as_millis(),
        "← réponse envoyée"
    );

    let valeur = HeaderValue::from_str(&rid).expect("un ULID ne contient que de l'ASCII");
    response.headers_mut().insert("x-request-id", valeur);
    response
}

/// `-` plutôt qu'un champ absent : une colonne qui disparaît une ligne sur deux
/// casse la lecture en terminal, et `coach=-` se distingue d'un coup d'œil.
fn nom_du_coach(auth_session: &AuthSession) -> String {
    auth_session
        .user
        .as_ref()
        .map(|u| u.coach_name.clone().into_inner())
        .unwrap_or_else(|| "-".to_string())
}
