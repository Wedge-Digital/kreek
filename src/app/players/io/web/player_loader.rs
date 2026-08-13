//! Chargement d'un joueur depuis la couche web.
//!
//! Ce fichier portait le contrôle d'appartenance à l'espace (carte 315). Il est
//! passé au middleware commun `web::middleware::space_scope` (carte 324) : une
//! règle valable pour huit BCs n'a pas à être réécrite dans chacun, sous peine
//! de diverger sans que personne ne s'en aperçoive.
//!
//! Il ne reste ici que le chargement, gardé en un point unique pour que la
//! traduction « joueur absent → 404 » et la journalisation des erreurs ne
//! soient pas recopiées dans les six handlers qui en ont besoin.

use crate::app::players::domain::player::{Player, PlayerId};
use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub async fn charger_joueur(state: &AppState, player_id: &str) -> Result<Player, Response> {
    match state
        .players
        .repository
        .find_by_id(&PlayerId(player_id.to_string()))
        .await
    {
        Ok(Some(p)) => Ok(p),
        Ok(None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("player_loader find_by_id {player_id}: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}
