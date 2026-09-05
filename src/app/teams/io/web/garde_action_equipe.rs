//! Qui peut agir sur une équipe — posé une fois, sur un groupe de routes.
//!
//! # Pourquoi un middleware et non dix-huit gardes de handler
//!
//! La règle est déjà écrite : `roster_edit_access_service::peut_modifier_effectif`.
//! Ce fichier ne la rejoue pas, il la **branche** — sur les dix-huit routes de
//! recrutement, de renvois, d'erreurs coûteuses et de validation de phase, qui
//! l'ignoraient toutes sauf une.
//!
//! Poser la garde en tête de chaque handler aurait demandé dix-huit copies, et
//! les handlers de mutation n'offrent même pas de point de passage commun :
//! `add_player` appelle `basket_mutation::add_player` **avant** de rendre son
//! fragment, donc avant de toucher le `charger()` que les autres partagent. Une
//! garde posée là aurait refusé l'affichage d'un panier déjà modifié.
//!
//! Surtout, dix-huit copies sont dix-huit occasions d'en oublier une : le
//! prochain widget de recrutement naîtrait ouvert, et rien ne le signalerait.
//! Le groupe de routes, lui, garde par construction ce qu'on y ajoute.
//!
//! # Ce qu'il ne garde pas, et pourquoi
//!
//! La fiche d'équipe, sa trésorerie et ses matchs se lisent par tout le monde —
//! la carte 500 y retire les boutons, pas la page. Le renvoi d'équipe et les
//! actions d'inscription relèvent d'une **autre** règle, celle du commissaire
//! (`SpacePermissions::is_admin()`), qui exclut délibérément le propriétaire.
//!
//! # La limite
//!
//! Il lit `team_id` dans le **chemin**, comme `space_scope_middleware`. Une
//! route qui désignerait son équipe par une chaîne de requête ou un champ de
//! formulaire lui échapperait sans que rien ne le dise. Aucune ne le fait
//! aujourd'hui : `routes.rs` porte `{team_id}` dans les dix-huit.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::teams::use_cases::roster_edit_access_service;
use crate::state::AppState;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::RequestPartsExt;
use std::collections::HashMap;

pub async fn garde_action_equipe(
    State(state): State<AppState>,
    auth_session: AuthSession,
    request: Request,
    next: Next,
) -> Response {
    let (mut parts, corps) = request.into_parts();

    let team_id = match parts.extract::<Path<HashMap<String, String>>>().await {
        Ok(Path(params)) => params.get("team_id").cloned(),
        Err(_) => None,
    };

    if let Err(refus) = autoriser(&state, &auth_session, team_id.as_deref()).await {
        return refus;
    }
    next.run(Request::from_parts(parts, corps)).await
}

/// **Une route de ce groupe sans `{team_id}` est refusée**, et non laissée
/// passer. Le cas ne peut venir que d'une route mal placée dans le groupe : la
/// laisser filer ouvrirait une action sans que personne ne s'en aperçoive.
async fn autoriser(
    state: &AppState,
    auth_session: &AuthSession,
    team_id: Option<&str>,
) -> Result<(), Response> {
    let Some(user) = auth_session.user.as_ref() else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };
    let Some(team_id) = team_id else {
        tracing::error!("garde_action_equipe : route sans team_id dans le chemin");
        return Err(StatusCode::BAD_REQUEST.into_response());
    };

    let team = match state.teams.team_repository.find_by_id(team_id).await {
        Ok(Some(team)) => team,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("garde_action_equipe : chargement de {team_id} : {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };

    let autorise = roster_edit_access_service::peut_modifier_effectif(
        &team,
        &user.id,
        &user.coach_name.clone().into_inner(),
        state.teams.access_port.as_ref(),
    )
    .await;

    match autorise {
        true => Ok(()),
        false => {
            tracing::warn!(
                team_id = %team_id,
                user_id = %user.id,
                "action refusée : ni propriétaire de l'équipe, ni administrateur"
            );
            Err(StatusCode::FORBIDDEN.into_response())
        }
    }
}
