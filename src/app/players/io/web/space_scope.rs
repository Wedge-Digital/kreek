//! Le joueur visé appartient-il bien à l'espace du chemin ?
//!
//! Question préalable à toute autorisation : il ne s'agit pas de savoir qui a
//! le droit, mais **de quoi on parle**. Les deux fonctions d'autorisation de ce
//! BC — `can_customise` et `can_spend_spp` — interrogent
//! `find_member_profile(user, space_id)` avec le `space_id` du **chemin**, donc
//! une valeur que l'appelant choisit. Sans ce contrôle, être admin d'un espace
//! quelconque suffit à agir sur n'importe quel joueur de l'application.
//!
//! La branche « admin de compétition » de ces deux fonctions est saine : elle
//! part de l'équipe du joueur. C'est la branche « admin d'espace » qui fait
//! confiance à l'URL, et c'est ici qu'on la ramène à la raison.
//!
//! **`404` et non `403`** : un `403` confirmerait l'existence d'un joueur d'un
//! autre espace, ce qu'un appelant qui énumère cherche précisément à apprendre.
//! Pour lui, ce joueur n'existe pas.

use crate::app::players::domain::player::{Player, PlayerId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Charge le joueur **et** vérifie qu'il vit dans l'espace du chemin.
///
/// Le seul moyen d'obtenir un `Player` depuis un handler de ce BC : passer par
/// ailleurs, c'est rouvrir la porte.
pub async fn charger_joueur_de_l_espace(
    state: &AppState,
    space_id: &str,
    player_id: &str,
) -> Result<Player, Response> {
    let Ok(space_id_vo) = SpaceId::try_new(space_id) else {
        return Err(StatusCode::BAD_REQUEST.into_response());
    };

    let player = match state
        .players
        .repository
        .find_by_id(&PlayerId(player_id.to_string()))
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("space_scope find_by_id {player_id}: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };

    match player.space_id == space_id_vo {
        true => Ok(player),
        false => {
            tracing::warn!(
                "space_scope : joueur {player_id} demandé depuis l'espace {space_id}, \
                 il appartient à {}",
                player.space_id
            );
            Err(StatusCode::NOT_FOUND.into_response())
        }
    }
}
