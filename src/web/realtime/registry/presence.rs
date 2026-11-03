//! La présence d'une connexion sur un match (carte 644).
//!
//! Sous-module du registre pour lire ses champs privés, et séparé de lui pour
//! que la présence et le routage des notifications (645) ne se marchent pas
//! dessus dans un même fichier.
//!
//! **Le rôle inscrit est celui que le serveur a calculé** : ce module ne le
//! décide pas, il le range. C'est `presence_controller` qui le calcule.

use super::{ConnectionId, ConnectionRegistry, Delivery};
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::{EntityId, UserId};
use crate::web::ports::MatchRole;
use crate::web::realtime::message::RealtimeMessage;

/// Le match qu'un onglet regarde, et ce qu'il y fait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presence {
    pub match_report_id: EntityId,
    pub role: MatchRole,
}

/// Les matchs dont les présents doivent apprendre qu'un onglet est arrivé,
/// parti ou a changé de rôle. Vide quand rien n'a changé.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PresenceChange {
    pub matches_to_notify: Vec<EntityId>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PresenceError {
    /// Connexion inconnue, ou d'un autre utilisateur — indiscernables pour
    /// l'appelant : rien n'est révélé de l'onglet d'autrui.
    UnknownConnection,
}

/// Une connexion présente sur un match, telle que le widget la lit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentConnection {
    pub id: ConnectionId,
    pub user_id: UserId,
    pub coach_name: CoachName,
    pub role: MatchRole,
}

impl PresenceChange {
    fn between(previous: Option<Presence>, next: Option<Presence>) -> Self {
        if previous == next {
            return Self::default();
        }
        let mut matches: Vec<EntityId> = previous
            .into_iter()
            .chain(next)
            .map(|p| p.match_report_id)
            .collect();
        matches.dedup();
        Self {
            matches_to_notify: matches,
        }
    }
}

impl ConnectionRegistry {
    /// Pose, remplace ou retire (`None`) la présence d'une connexion — à
    /// condition qu'elle appartienne à `user_id`.
    pub fn set_presence(
        &self,
        id: &ConnectionId,
        user_id: &UserId,
        presence: Option<Presence>,
    ) -> Result<PresenceChange, PresenceError> {
        let mut connection = self
            .connections
            .get_mut(id)
            .filter(|c| &c.user_id == user_id)
            .ok_or(PresenceError::UnknownConnection)?;
        let previous = std::mem::replace(&mut connection.presence, presence);
        Ok(PresenceChange::between(previous, presence))
    }

    /// Les connexions présentes sur ce match, tous utilisateurs confondus.
    pub fn present_on(&self, match_report_id: &EntityId) -> Vec<PresentConnection> {
        self.connections
            .iter()
            .filter_map(|entry| {
                let presence = entry
                    .presence
                    .filter(|p| &p.match_report_id == match_report_id)?;
                Some(PresentConnection {
                    id: *entry.key(),
                    user_id: entry.user_id,
                    coach_name: entry.coach_name.clone(),
                    role: presence.role,
                })
            })
            .collect()
    }

    /// Prévient les présents d'un match que sa présence a changé, sauf
    /// `except` — l'onglet qui vient de se déclarer n'a rien à recharger.
    ///
    /// Les destinataires sont collectés avant toute livraison : aucun verrou du
    /// registre n'est tenu pendant un `deliver`.
    pub fn notify_presence_changed(
        &self,
        match_report_id: &EntityId,
        except: Option<ConnectionId>,
    ) {
        let message = RealtimeMessage::PresenceChanged {
            match_report_id: *match_report_id,
        };
        for present in self.present_on(match_report_id) {
            if Some(present.id) != except {
                self.deliver(&present.id, message.clone());
            }
        }
    }

    /// Prévient **tous** les présents d'un match — l'auteur du fait compris —
    /// que ce match a changé (carte 646). Rend le nombre de files qui ont
    /// accepté le signal.
    ///
    /// Être présent suffit : la présence n'est inscrite qu'avec un rôle
    /// calculé par `match_role`, donc avec le droit de suivre (R6).
    pub fn notify_match_changed(&self, match_report_id: &EntityId) -> usize {
        let message = RealtimeMessage::MatchChanged {
            match_report_id: *match_report_id,
        };
        self.present_on(match_report_id)
            .iter()
            .filter(|present| self.deliver(&present.id, message.clone()) == Delivery::Queued)
            .count()
    }
}
