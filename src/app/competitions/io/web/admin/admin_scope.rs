//! La cible d'une mutation d'administration appartient-elle à la saison du
//! chemin ?
//!
//! # Pourquoi ces contrôles sont nécessaires
//!
//! `space_scope` ne résout que les paramètres de **chemin**, et seulement ceux
//! pour lesquels un BC a posé un résolveur — `competition_id`, `season_id`,
//! `team_id`, `player_id`, `match_report_id`, `article_id`. Sa docstring dit du
//! reste :
//!
//! > Les paramètres sans résolveur (`round_id`, `pairing_id`, `action_id`…)
//! > passent : ils sont toujours accompagnés d'un parent qui, lui, est contrôlé.
//!
//! La prémisse est vraie, la conclusion ne l'est pas. Le parent est contrôlé ;
//! **rien ne rattache l'enfant au parent**. Un `round_id` reste libre, dans le
//! chemin comme dans le corps — le déplacer de l'un à l'autre ne change que la
//! forme, jamais le contrôle. C'est ce que la carte 416 croyait pouvoir faire,
//! et c'est pourquoi ces trois fonctions existent.
//!
//! # `404`, jamais `403`
//!
//! Une cible qui n'appartient pas à la saison du chemin est hors de ce que le
//! chemin désigne. Un `403` confirmerait son existence à qui se contente
//! d'essayer des identifiants ; un `404` ne dit rien de plus que « pas ici ».

use crate::app::competitions::domain::match_day::MatchDay;
use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// La journée visée, si elle appartient bien à cette saison.
///
/// Rend l'agrégat plutôt qu'un booléen : quatre des cinq appelants en avaient
/// besoin de toute façon, et le leur rendre ici leur épargne une seconde
/// lecture — un contrôle qui coûte une requête de plus se contourne un jour
/// « pour la performance ».
pub async fn journee_de_la_saison(
    round_id: &str,
    season_id: &str,
    state: &AppState,
) -> Result<MatchDay, Response> {
    match state
        .competitions
        .match_day_repository
        .find_by_id(round_id)
        .await
    {
        Ok(Some(journee)) if journee.season_id.to_string() == season_id => Ok(journee),
        Ok(_) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("journee_de_la_saison {round_id}: {e:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// L'appariement visé appartient-il à cette saison ?
///
/// Passe par les journées de la saison : un appariement ne porte pas sa saison,
/// il n'est connu que de la journée qui le contient. Une lecture, et un parcours
/// borné par la taille du calendrier.
pub async fn appariement_de_la_saison(
    pairing_id: &str,
    season_id: &str,
    state: &AppState,
) -> Result<(), Response> {
    let journees = match state
        .competitions
        .match_day_repository
        .find_by_season(season_id)
        .await
    {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("appariement_de_la_saison {pairing_id}: {e:?}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    let connu = journees
        .iter()
        .flat_map(|j| j.pairings.iter())
        .any(|p| p.id.to_string() == pairing_id);
    match connu {
        true => Ok(()),
        false => Err(StatusCode::NOT_FOUND.into_response()),
    }
}

/// Le groupe visé appartient-il à cette saison ?
pub async fn groupe_de_la_saison(
    group_id: &str,
    season_id: &str,
    state: &AppState,
) -> Result<(), Response> {
    match state
        .competitions
        .group_repository
        .find_groups(season_id)
        .await
    {
        Ok(groupes) if groupes.iter().any(|g| g.group_id == group_id) => Ok(()),
        Ok(_) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("groupe_de_la_saison {group_id}: {e:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// L'équipe visée est-elle inscrite à cette saison ?
///
/// Sans quoi un administrateur légitime pourrait affecter dans ses propres
/// poules une équipe qui n'a rien à y faire — le contrôle d'accès serait
/// satisfait, la donnée non.
pub async fn equipe_de_la_saison(
    team_id: &str,
    season_id: &str,
    state: &AppState,
) -> Result<(), Response> {
    let inscrites = match state
        .competitions
        .team_info_port
        .find_enrolled_teams(season_id)
        .await
    {
        Ok(equipes) => equipes,
        Err(e) => {
            tracing::error!("equipe_de_la_saison {team_id}: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    match inscrites.iter().any(|t| t.team_id == team_id) {
        true => Ok(()),
        false => Err(StatusCode::NOT_FOUND.into_response()),
    }
}
