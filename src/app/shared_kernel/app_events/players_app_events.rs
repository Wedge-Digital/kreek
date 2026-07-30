use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};

/// Franchit la frontière `players` → `teams`.
///
/// `InitialRosterCompleted` résout une course : les deux BCs s'abonnent au même
/// app event `TeamCreated`, dans deux tâches indépendantes. Sans ce signal,
/// `teams` peut atteindre `ReadyToPlay` et recalculer sa valeur avant que
/// `players` ait inséré le moindre joueur — la TV vaudrait alors zéro, ou pire
/// une valeur partielle, plausible et fausse.
///
/// `teams` pourrait valoriser lui-même le payload de `TeamCreated`, qui porte
/// déjà lignes de roster et compétences. Écarté : il réimplémenterait la règle
/// de valorisation de `players`, la duplication même qui avait produit les deux
/// tables divergentes corrigées par la carte 249.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayersAppEvent {
    InitialRosterCompleted {
        team_id: String,
        player_count: u32,
    },
    /// Le joueur a quitté l'effectif — **écrit**, pas seulement décidé.
    ///
    /// `teams` décide du renvoi, mais c'est `players` qui possède l'effectif :
    /// tant qu'il n'a pas écrit, une valeur d'équipe recalculée compterait
    /// encore le partant. Cet événement est le seul instant où l'information
    /// existe, et c'est pour ça qu'il fait le trajet retour. Même raison, même
    /// forme qu'`InitialRosterCompleted`, qui existe pour la course jumelle à
    /// la création d'équipe.
    PlayerDismissed {
        team_id: String,
        player_id: String,
    },
}

impl PlayersAppEvent {
    pub const INITIAL_ROSTER_COMPLETED: &'static str = "PlayersInitialRosterCompleted";
    pub const PLAYER_DISMISSED: &'static str = "PlayersPlayerDismissed";

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::InitialRosterCompleted { .. } => Self::INITIAL_ROSTER_COMPLETED,
            Self::PlayerDismissed { .. } => Self::PLAYER_DISMISSED,
        }
    }

    pub fn to_enveloppe(&self) -> EventEnvelope {
        let emitter = match self {
            // L'émetteur est l'équipe : c'est elle que le listener de `teams`
            // recalculera, pas le joueur.
            Self::InitialRosterCompleted { team_id, .. }
            | Self::PlayerDismissed { team_id, .. } => team_id.clone(),
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
