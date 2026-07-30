use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::{EventId, SpaceId};
use crate::common::event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

/// Franchit la frontière `teams` → `players`.
///
/// Recruter est un fait de `teams` ; l'entité joueur vit dans `players`. Sans
/// cet événement, le coach paie un joueur qui n'existe nulle part.
///
/// L'identifiant du joueur est frappé par `teams`, dans son événement domaine :
/// l'event store devient la source d'identité, ce qui rend l'opération
/// rejouable et permet à la contrainte d'unicité de `players` de rejeter un
/// doublon au lieu de créer un second joueur.
///
/// Le coût n'y figure pas : c'est une affaire de trésorerie, qui ne regarde
/// pas `players`. Le nom du poste non plus — `players` le résout depuis la
/// ligne de roster, par son propre catalogue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TeamsAppEvent {
    PlayerRecruited {
        event_id: EventId,
        team_id: TeamId,
        space_id: SpaceId,
        player_id: PlayerId,
        roster_line_id: String,
        base_value_kpo: u32,
    },
    /// Le coach a renvoyé ce joueur. `players` le sort de son effectif.
    ///
    /// Rien d'autre à transporter : ni valeur ni motif. `players` possède le
    /// joueur, il sait tout de lui ; ce qu'il ignorait, c'est la décision.
    PlayerDismissed {
        event_id: EventId,
        team_id: TeamId,
        space_id: SpaceId,
        player_id: PlayerId,
    },
}

impl TeamsAppEvent {
    pub const PLAYER_RECRUITED: &'static str = "TeamsPlayerRecruited";
    pub const PLAYER_DISMISSED: &'static str = "TeamsPlayerDismissed";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::PlayerRecruited { .. } => Self::PLAYER_RECRUITED,
            Self::PlayerDismissed { .. } => Self::PLAYER_DISMISSED,
        }
    }

    /// L'émetteur est l'équipe, pas le joueur : c'est elle qui a recruté, et
    /// c'est par elle que le listener retrouve le contexte.
    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            Self::PlayerRecruited { team_id, .. } | Self::PlayerDismissed { team_id, .. } => {
                team_id.to_string()
            }
        };
        EventEnvelope {
            event_id: EventId::new().to_string(),
            emitter,
            event_type: self.event_type().to_string(),
            tags: serde_json::json!([]),
            payload: serde_json::to_value(self).unwrap(),
            occurred_at: time::OffsetDateTime::now_utc(),
        }
    }
}
