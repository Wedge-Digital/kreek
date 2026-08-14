//! Une ressource n'est atteignable que depuis son espace.
//!
//! Empêche qu'un admin de l'espace A lise ou modifie une ressource de l'espace
//! B en mettant son propre espace dans l'URL. Le défaut a été **prouvé** sur
//! quatre BCs (carte 316), en lecture comme en écriture.
//!
//! # Un mécanisme, un résolveur par ressource
//!
//! Ce qui varie d'un BC à l'autre est mince : quel paramètre de chemin désigne
//! une ressource, et comment remonter à son espace. Tout le reste — lire le
//! chemin, comparer, choisir le code de retour — est identique. Écrite cinq
//! fois, cette sémantique divergerait, et un garde qui diverge est un garde
//! inutile dont personne ne s'aperçoit.
//!
//! Chaque BC répond **sur ses propres ressources, via son propre repository**.
//! Un middleware qui interrogerait les tables directement violerait la
//! souveraineté des données.
//!
//! # Ce qu'il ne voit pas
//!
//! **Uniquement les paramètres de chemin.** Une ressource désignée par une
//! chaîne de requête ou un champ de formulaire lui échappe, sans que rien ne le
//! signale. Aujourd'hui sans conséquence — les `POST` qui reçoivent un
//! identifiant dans leur corps portent toujours leur parent dans le chemin,
//! lui contrôlé — mais la limite doit être connue avant qu'un cas dangereux
//! n'apparaisse.
//!
//! Les paramètres sans résolveur (`round_id`, `pairing_id`, `action_id`…)
//! passent : ils sont toujours accompagnés d'un parent qui, lui, est contrôlé.

use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use async_trait::async_trait;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::RequestPartsExt;
use std::collections::HashMap;
use std::sync::Arc;

/// La question que le middleware pose, et qu'un seul BC sait résoudre.
#[async_trait]
pub trait ISpaceOwnership: Send + Sync {
    /// Le paramètre de chemin que ce résolveur sait traiter — « player_id ».
    fn param(&self) -> &'static str;

    /// À quel espace appartient cette ressource ? `None` si elle n'existe pas,
    /// ce que le middleware traite comme un `404` — au même titre qu'une
    /// ressource d'un autre espace.
    async fn space_of(&self, id: &str) -> Option<SpaceId>;
}

/// **`404` et non `403`** pour une ressource étrangère : un `403` confirmerait
/// son existence à qui l'énumère. Pour lui, elle n'existe pas.
///
/// Le contrôle vient **avant** l'autorisation : il ne s'agit pas de savoir qui
/// a le droit, mais de quoi on parle.
pub async fn space_scope_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let (mut parts, corps) = request.into_parts();

    let params = match parts.extract::<Path<HashMap<String, String>>>().await {
        Ok(Path(p)) => p,
        // Route sans paramètre nommé : rien à contrôler.
        Err(_) => return next.run(Request::from_parts(parts, corps)).await,
    };

    if let Err(refus) = verifier(&state, &params).await {
        return refus;
    }
    next.run(Request::from_parts(parts, corps)).await
}

/// Deux résolveurs pour le même paramètre : le second ne serait jamais
/// consulté, et le premier déciderait pour un BC qui ne le sait pas.
///
/// Vécu (cartes 320 et 321) : `{team_id}` est revendiqué par `teams` et par
/// `team_creation`. Un résolveur lisant la seule projection a rendu `404` sur
/// tous les brouillons non soumis, cassant la création d'équipe — sans qu'aucun
/// test unitaire ne bronche.
///
/// Un doublon est donc une **erreur de démarrage**, et non un arbitrage
/// silencieux. La bonne réponse est un résolveur qui consulte les deux sources,
/// comme `TeamSpaceOwnership` le fait désormais.
pub fn verifier_unicite_des_parametres(resolveurs: &[Arc<dyn ISpaceOwnership>]) {
    let mut vus: Vec<&str> = resolveurs.iter().map(|r| r.param()).collect();
    vus.sort_unstable();
    let total = vus.len();
    vus.dedup();
    assert_eq!(
        vus.len(),
        total,
        "deux résolveurs revendiquent le même paramètre de chemin — l'un d'eux \
         ne serait jamais consulté"
    );
}

async fn verifier(state: &AppState, params: &HashMap<String, String>) -> Result<(), Response> {
    // Pas d'espace dans le chemin : la route n'est pas de celles que ce
    // middleware protège.
    let Some(brut) = params.get("space_id") else {
        return Ok(());
    };
    let Ok(space_id) = SpaceId::try_new(brut) else {
        return Err(StatusCode::BAD_REQUEST.into_response());
    };

    for resolveur in state.space_ownership.iter() {
        let Some(id) = params.get(resolveur.param()) else {
            continue;
        };
        match resolveur.space_of(id).await {
            Some(reel) if reel == space_id => continue,
            _ => {
                tracing::warn!(
                    "space_scope : {} = {id} demandé depuis l'espace {brut}, refusé",
                    resolveur.param()
                );
                return Err(StatusCode::NOT_FOUND.into_response());
            }
        }
    }
    Ok(())
}
