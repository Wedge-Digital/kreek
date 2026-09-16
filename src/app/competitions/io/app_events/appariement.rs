//! Résolution de l'appariement d'un rapport de match, partagée par les
//! listeners de `competitions` qui en ont besoin.
//!
//! **Une seule fonction, et elle ne fait que chercher** (carte 555).
//!
//! Il y en avait deux : celle-ci, et une `resoudre_ou_creer_appariement` qui
//! fabriquait l'appariement manquant d'un rapport manuel. Ce cas n'existe plus
//! — un rapport naît toujours d'un appariement et en porte l'identifiant dès
//! son premier événement. Fabriquer encore aurait produit un second
//! appariement pour la même rencontre, ce que la carte 552 a effectivement
//! provoqué.
//!
//! Le contexte est une struct plutôt que le payload d'un événement : ses
//! appelants reçoivent des événements de formes différentes, et seuls ces sept
//! champs comptent.

use crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportPublishedPayload;

/// Ce qu'il faut savoir d'un rapport pour retrouver son appariement.
///
/// `pairing_id` à `None` désignait un rapport manuel, du temps où il pouvait
/// naître sans appariement. Le champ reste pour relire l'historique — les
/// événements d'alors le portent à `null`.
pub struct ContexteAppariement {
    pub match_report_id: String,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub round_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub pairing_id: Option<String>,
}

impl ContexteAppariement {
    /// Depuis le payload de publication, dont il reprend les sept champs utiles.
    pub fn depuis_publication(p: &MatchReportPublishedPayload) -> Self {
        Self {
            match_report_id: p.match_report_id.clone(),
            space_id: p.space_id.clone(),
            competition_id: p.competition_id.clone(),
            season_id: p.season_id.clone(),
            round_id: p.round_id.clone(),
            home_team_id: p.home_team_id.clone(),
            away_team_id: p.away_team_id.clone(),
            pairing_id: p.pairing_id.clone(),
        }
    }
}

/// La **recherche seule** : l'appariement porté par le rapport, sinon celui qui
/// existerait déjà pour ce trio journée/domicile/extérieur.
///
/// C'est ce dont la dépublication a besoin, et rien de plus.
pub async fn trouver_appariement(
    payload: &ContexteAppariement,
    match_day_repo: &dyn IMatchDayRepository,
) -> Option<String> {
    if let Some(pairing_id) = &payload.pairing_id {
        return Some(pairing_id.clone());
    }
    match match_day_repo
        .find_pairing_id(
            &payload.round_id,
            &payload.home_team_id,
            &payload.away_team_id,
        )
        .await
    {
        Ok(existant) => existant,
        Err(e) => {
            tracing::error!("appariement: recherche du pairing existant : {e}");
            None
        }
    }
}
