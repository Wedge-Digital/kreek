//! Résolution de l'appariement d'un rapport de match, partagée par les trois
//! listeners de `competitions` qui en ont besoin.
//!
//! **Deux fonctions et non une**, parce que les contrats diffèrent : la
//! publication et la confirmation créent l'appariement manquant d'un rapport
//! manuel, la dépublication ne le fait jamais — il existe forcément déjà, et
//! elle ne ferait que le dupliquer. Une fonction unique aurait demandé un
//! drapeau, donc un comportement suspendu à un booléen.
//!
//! Le contexte est une struct plutôt que le payload d'un événement : les trois
//! appelants reçoivent des événements de formes différentes, et seuls ces sept
//! champs comptent.

use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, NewPairingProjection,
};
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::competitions::use_cases::admin::team_enrollment::load_enrolled_teams;
use crate::app::shared_kernel::app_events::match_report_app_events::MatchReportPublishedPayload;
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

/// Ce qu'il faut savoir d'un rapport pour lui trouver — ou lui fabriquer — un
/// appariement. `pairing_id` à `None` désigne un rapport manuel.
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

/// Un rapport "manuel" (saisi hors calendrier) n'a pas de pairing — la ligne de
/// projection résultats/calendrier n'existe donc pas encore à ce stade. On crée
/// alors un vrai pairing pour la journée déjà choisie à la création du rapport,
/// exactement comme le ferait "Ajouter un match" : la ligne de projection est
/// insérée en synchrone (même tâche) pour éviter toute course avec l'UPDATE qui
/// suit, et l'event PairingCreated est aussi émis pour la cohérence du système
/// (ex. suivi des équipes déjà affrontées côté génération automatique).
pub async fn resoudre_ou_creer_appariement(
    payload: &ContexteAppariement,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Option<String> {
    if let Some(pairing_id) = &payload.pairing_id {
        return Some(pairing_id.clone());
    }

    // Un rapport manuel republié après correction retomberait ici : sans cette
    // recherche, on créerait un second pairing pour le même match, qui
    // apparaîtrait alors deux fois au calendrier.
    match match_day_repo
        .find_pairing_id(
            &payload.round_id,
            &payload.home_team_id,
            &payload.away_team_id,
        )
        .await
    {
        Ok(Some(existing)) => return Some(existing),
        Ok(None) => {}
        Err(e) => {
            tracing::error!("appariement: recherche du pairing existant : {e}");
            return None;
        }
    }

    let match_day = match match_day_repo.find_by_id(&payload.round_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            tracing::error!(
                "appariement: journée {} introuvable pour le rapport manuel {}",
                payload.round_id,
                payload.match_report_id
            );
            return None;
        }
        Err(e) => {
            tracing::error!("appariement: find_by_id journée {}: {e}", payload.round_id);
            return None;
        }
    };

    let team_display = match load_enrolled_teams(&payload.season_id, team_port).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("appariement: load_enrolled_teams: {e}");
            return None;
        }
    };
    let Some(home_info) = team_display.get(&payload.home_team_id) else {
        tracing::error!(
            "appariement: équipe domicile {} non enrôlée pour le rapport manuel {}",
            payload.home_team_id,
            payload.match_report_id
        );
        return None;
    };
    let Some(away_info) = team_display.get(&payload.away_team_id) else {
        tracing::error!(
            "appariement: équipe extérieure {} non enrôlée pour le rapport manuel {}",
            payload.away_team_id,
            payload.match_report_id
        );
        return None;
    };

    let (Ok(home_team_id), Ok(away_team_id)) = (
        TeamId::try_new(&payload.home_team_id),
        TeamId::try_new(&payload.away_team_id),
    ) else {
        tracing::error!(
            "appariement: id d'équipe invalide pour {}",
            payload.match_report_id
        );
        return None;
    };
    let pairing = Pairing {
        id: PairingId::new(),
        home_team_id,
        away_team_id,
    };
    let projection = NewPairingProjection {
        season_id: payload.season_id.clone(),
        round_name: match_day.name.to_string(),
        round_position: match_day.position.into_inner(),
        round_date_start: match_day.date_start.as_ref().map(|d| d.to_string()),
        round_date_end: match_day.date_end.as_ref().map(|d| d.to_string()),
        round_day_type: match_day.day_type.as_str().to_string(),
        home_team_name: home_info.team_name.clone(),
        home_roster_name: home_info.roster_name.clone(),
        home_coach_name: home_info.coach_name.clone(),
        home_logo_url: home_info.logo_url.clone(),
        away_team_name: away_info.team_name.clone(),
        away_roster_name: away_info.roster_name.clone(),
        away_coach_name: away_info.coach_name.clone(),
        away_logo_url: away_info.logo_url.clone(),
    };
    if let Err(e) = match_day_repo
        .save_pairing(&payload.round_id, &pairing, &projection)
        .await
    {
        tracing::error!("appariement: save_pairing: {e}");
        return None;
    }

    let pairing_id = pairing.id.to_string();

    emettre(
        event_bus,
        CompetitionsDomainEvent::PairingCreated {
            event_id: EventId::new(),
            pairing_id: pairing_id.clone(),
            competition_id: payload.competition_id.clone(),
            season_id: payload.season_id.clone(),
            round_id: payload.round_id.clone(),
            home_team_id: payload.home_team_id.clone(),
            away_team_id: payload.away_team_id.clone(),
            space_id: payload.space_id.clone(),
            home_team_name: home_info.team_name.clone(),
            home_roster_name: home_info.roster_name.clone(),
            home_coach_name: home_info.coach_name.clone(),
            home_logo_url: home_info.logo_url.clone(),
            away_team_name: away_info.team_name.clone(),
            away_roster_name: away_info.roster_name.clone(),
            away_coach_name: away_info.coach_name.clone(),
            away_logo_url: away_info.logo_url.clone(),
            round_name: match_day.name.to_string(),
            round_position: match_day.position.into_inner(),
            round_date_start: match_day.date_start.as_ref().map(|d| d.to_string()),
            round_date_end: match_day.date_end.as_ref().map(|d| d.to_string()),
            round_day_type: match_day.day_type.as_str().to_string(),
        }
        .to_enveloppe(),
    );

    Some(pairing_id)
}
