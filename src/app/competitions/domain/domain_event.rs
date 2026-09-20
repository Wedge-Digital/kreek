use crate::app::shared_kernel::app_events::competitions_app_events::CompetitionsAppEvent;
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, EventId, SpaceId};
use crate::common::event_envelope::EventEnvelope;
use crate::common::services::event_bus::event_tags::{EventTag, EventTagName};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CompetitionsDomainEvent {
    CompetitionCreated {
        event_id: EventId,
        competition_id: CompetitionId,
        space_id: SpaceId,
        created_by: CoachId,
        name: CompetitionName,
        logo: CloudinaryImage,
        admin_ids: Vec<CoachId>,
    },
    CompetitionReady {
        event_id: EventId,
        competition_id: CompetitionId,
        space_id: SpaceId,
        finalized_by: CoachId,
    },
    PairingCreated {
        event_id: EventId,
        pairing_id: String,
        competition_id: String,
        season_id: String,
        round_id: String,
        home_team_id: String,
        away_team_id: String,
        space_id: String,
        home_team_name: String,
        home_roster_name: String,
        home_coach_name: String,
        home_logo_url: Option<String>,
        away_team_name: String,
        away_roster_name: String,
        away_coach_name: String,
        away_logo_url: Option<String>,
        round_name: String,
        round_position: i32,
        round_date_start: Option<String>,
        round_date_end: Option<String>,
        round_day_type: String,
    },
    /// Une rencontre existe qui n'était **pas au calendrier** (carte 555).
    ///
    /// Même charge utile que [`Self::PairingCreated`], et un nom différent pour
    /// une seule raison : il ne doit pas faire créer de rapport. Sur ce chemin,
    /// c'est le contrôleur qui le crée, au retour de l'appel — s'il partait
    /// aussi par l'événement, on en aurait deux.
    ///
    /// Le nom dit un fait du domaine, pas son déclencheur :
    /// `PairingCreatedFromMatchReport` aurait trahi l'origine, ce que la
    /// convention proscrit.
    ///
    /// Il ne sort pas du BC — voir le bras groupé de [`Self::to_app_event`] —
    /// mais il est persisté : l'incident du 15 septembre a demandé de croiser
    /// des ULID et des horodatages pour établir qu'un appariement avait été
    /// fabriqué hors calendrier, et c'est désormais un `grep` dans `event_log`.
    OutOfSchedulePairingCreated {
        event_id: EventId,
        pairing_id: String,
        competition_id: String,
        season_id: String,
        round_id: String,
        home_team_id: String,
        away_team_id: String,
        space_id: String,
        home_team_name: String,
        home_roster_name: String,
        home_coach_name: String,
        home_logo_url: Option<String>,
        away_team_name: String,
        away_roster_name: String,
        away_coach_name: String,
        away_logo_url: Option<String>,
        round_name: String,
        round_position: i32,
        round_date_start: Option<String>,
        round_date_end: Option<String>,
        round_day_type: String,
    },
    PairingDeleted {
        event_id: EventId,
        pairing_id: String,
    },
    /// Une rencontre a changé de journée (carte 557).
    ///
    /// Le nom de la journée d'arrivée voyage avec l'événement : `match_report`
    /// et `players` en ont besoin pour leurs libellés, et ils n'ont pas à
    /// relire le calendrier de `competitions` pour ça.
    PairingMoved {
        event_id: EventId,
        pairing_id: String,
        season_id: String,
        space_id: String,
        home_team_id: String,
        away_team_id: String,
        from_round_id: String,
        to_round_id: String,
        to_round_name: String,
    },
}

pub const COMPETITION_CREATED: &str = "CompetitionCreated";
pub const COMPETITION_READY: &str = "CompetitionReady";
pub const PAIRING_CREATED: &str = "PairingCreated";
pub const OUT_OF_SCHEDULE_PAIRING_CREATED: &str = "OutOfSchedulePairingCreated";
pub const PAIRING_DELETED: &str = "PairingDeleted";
pub const PAIRING_MOVED: &str = "PairingMoved";

impl CompetitionsDomainEvent {
    pub fn to_event_type(&self) -> &'static str {
        match self {
            Self::CompetitionCreated { .. } => COMPETITION_CREATED,
            Self::CompetitionReady { .. } => COMPETITION_READY,
            Self::PairingCreated { .. } => PAIRING_CREATED,
            Self::OutOfSchedulePairingCreated { .. } => OUT_OF_SCHEDULE_PAIRING_CREATED,
            Self::PairingDeleted { .. } => PAIRING_DELETED,
            Self::PairingMoved { .. } => PAIRING_MOVED,
        }
    }

    fn emitter_id(&self) -> String {
        match self {
            Self::CompetitionCreated { competition_id, .. } => competition_id.to_string(),
            Self::CompetitionReady { competition_id, .. } => competition_id.to_string(),
            Self::PairingCreated { pairing_id, .. }
            | Self::OutOfSchedulePairingCreated { pairing_id, .. }
            | Self::PairingDeleted { pairing_id, .. }
            | Self::PairingMoved { pairing_id, .. } => pairing_id.clone(),
        }
    }

    pub fn get_tags(&self) -> Vec<EventTag> {
        match self {
            Self::CompetitionCreated {
                space_id,
                competition_id,
                ..
            }
            | Self::CompetitionReady {
                space_id,
                competition_id,
                ..
            } => vec![
                EventTag {
                    name: EventTagName::Space,
                    value: space_id.to_string(),
                },
                EventTag {
                    name: EventTagName::Competition,
                    value: competition_id.to_string(),
                },
            ],
            Self::PairingCreated { .. }
            | Self::OutOfSchedulePairingCreated { .. }
            | Self::PairingDeleted { .. }
            | Self::PairingMoved { .. } => vec![],
        }
    }

    pub fn to_app_event(&self) -> Option<CompetitionsAppEvent> {
        match self {
            Self::CompetitionReady {
                competition_id,
                space_id,
                ..
            } => Some(CompetitionsAppEvent::CompetitionCreated {
                event_id: EventId::new(),
                competition_id: *competition_id,
                space_id: *space_id,
            }),
            Self::PairingCreated {
                pairing_id,
                competition_id,
                season_id,
                round_id,
                home_team_id,
                away_team_id,
                space_id,
                ..
            } => Some(CompetitionsAppEvent::PairingCreated {
                event_id: EventId::new(),
                pairing_id: pairing_id.clone(),
                competition_id: competition_id.clone(),
                season_id: season_id.clone(),
                round_id: round_id.clone(),
                home_team_id: home_team_id.clone(),
                away_team_id: away_team_id.clone(),
                space_id: space_id.clone(),
            }),
            Self::PairingDeleted { pairing_id, .. } => Some(CompetitionsAppEvent::PairingDeleted {
                event_id: EventId::new(),
                pairing_id: pairing_id.clone(),
            }),
            Self::PairingMoved {
                pairing_id,
                season_id,
                from_round_id,
                to_round_id,
                to_round_name,
                ..
            } => Some(CompetitionsAppEvent::PairingMoved {
                event_id: EventId::new(),
                pairing_id: pairing_id.clone(),
                season_id: season_id.clone(),
                from_round_id: from_round_id.clone(),
                to_round_id: to_round_id.clone(),
                to_round_name: to_round_name.clone(),
            }),
            // **Ceux qui ne sortent pas du BC**, nommés un par un.
            //
            // Ils remplacent un `_ => None` dont le commentaire disait lui-même qu'il
            // « avale silencieusement tout événement domaine qu'on oublierait de faire
            // sortir ». C'est arrivé : la carte 457 y a perdu un journalier payé.
            //
            // Sans joker, ajouter un variant casse la compilation ici — et son auteur
            // tranche : il sort, ou il rejoint cette liste (carte 506).
            //
            // `OutOfSchedulePairingCreated` en fait partie **par construction** : le
            // faire sortir ferait créer un second rapport par
            // `pairing_created_listener`, ce que la carte 555 supprime.
            Self::CompetitionCreated { .. } | Self::OutOfSchedulePairingCreated { .. } => None,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter: self.emitter_id(),
            event_type: self.to_event_type().to_string(),
            tags: serde_json::to_value(self.get_tags()).unwrap(),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
