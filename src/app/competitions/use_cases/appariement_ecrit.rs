//! Ce qu'on annonce quand une rencontre est écrite — pour les **deux** chemins.
//!
//! # L'événement se lit sur la projection, pas sur la table d'affichage
//!
//! `PairingCreated` porte vingt champs, dont quatorze sont **exactement** ceux de
//! `NewPairingProjection` : les noms d'équipe, de roster et de coach, les logos,
//! le nom et les dates de la journée. Les reconstruire depuis `team_display`
//! demandait deux `HashMap::get(...).expect(...)` sur un invariant tenu ailleurs —
//! le filtrage d'enrôlement fait avant l'appariement.
//!
//! Les lire sur la projection qu'on vient d'écrire supprime ces deux `expect` :
//! si la projection existe, ses noms existent. Et l'événement ne peut plus
//! diverger de ce que la base contient, puisque les deux sortent de la même
//! valeur.
//!
//! # L'émission vient après le commit, jamais dedans
//!
//! Un listener qui réagit à un `PairingCreated` dont la transaction est ensuite
//! annulée aurait travaillé sur un fait qui n'a pas eu lieu, et rien ne le lui
//! dirait. L'ordre inverse paraît plus naturel — « tout dans la même unité » — et
//! c'est le piège. Ce module ne peut pas le garantir ; les appelants s'en
//! chargent, et leurs tests le vérifient.

use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::NewPairingProjection;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

/// Les identifiants que la projection ne porte pas — elle connaît la saison, pas
/// la compétition, l'espace ni la journée.
pub struct OuEstEcrite<'a> {
    pub competition_id: &'a str,
    pub space_id: &'a str,
    pub round_id: &'a str,
}

/// **Toujours par `emettre()`.** Un envoi écrit à la main sur le bus reprendrait
/// l'identifiant de l'enveloppe reçue au lieu de celui que `to_enveloppe()`
/// engendre, et produirait une trace qui a l'air correcte sans rien corréler.
///
/// La formulation évite d'écrire le nom de la méthode du bus : l'axe 12 la cherche
/// sans retirer les commentaires, contrairement à l'axe 9 — un verrou qui hurle
/// sur sa propre documentation se fait désactiver dans la semaine.
pub fn emettre_pairing_created(
    bus: &EventBus,
    ou: OuEstEcrite<'_>,
    pairing: &Pairing,
    projection: &NewPairingProjection,
) {
    emettre(
        bus,
        CompetitionsDomainEvent::PairingCreated {
            event_id: EventId::new(),
            pairing_id: pairing.id.to_string(),
            competition_id: ou.competition_id.to_string(),
            season_id: projection.season_id.clone(),
            round_id: ou.round_id.to_string(),
            home_team_id: pairing.home_team_id.to_string(),
            away_team_id: pairing.away_team_id.to_string(),
            space_id: ou.space_id.to_string(),
            home_team_name: projection.home_team_name.clone(),
            home_roster_name: projection.home_roster_name.clone(),
            home_coach_name: projection.home_coach_name.clone(),
            home_logo_url: projection.home_logo_url.clone(),
            away_team_name: projection.away_team_name.clone(),
            away_roster_name: projection.away_roster_name.clone(),
            away_coach_name: projection.away_coach_name.clone(),
            away_logo_url: projection.away_logo_url.clone(),
            round_name: projection.round_name.clone(),
            round_position: projection.round_position,
            round_date_start: projection.round_date_start.clone(),
            round_date_end: projection.round_date_end.clone(),
            round_day_type: projection.round_day_type.clone(),
        }
        .to_enveloppe(),
    );
}
