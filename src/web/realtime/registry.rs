//! Le registre des connexions temps réel (carte 642, épic E23).
//!
//! Une connexion par onglet authentifié (R1). Le registre sait lesquelles sont
//! ouvertes et à qui elles appartiennent ; il vit **en mémoire**, local au
//! processus — aucune donnée n'en est écrite ailleurs.
//!
//! La livraison passe par `try_send` sur une file bornée : une file pleine perd
//! le message et le compte, elle ne bloque jamais l'émetteur (R8). Un client
//! lent perd ses messages ; il ne ralentit pas les autres.

use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::UserId;
use crate::app::shared_kernel::identity::sulid::SUlid;
use crate::web::realtime::message::RealtimeMessage;
use dashmap::DashMap;
use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

/// Messages en attente par connexion. Au-delà, ils sont perdus (R3, R8).
pub const QUEUE_CAPACITY: usize = 32;

/// L'identifiant d'une connexion, rendu au navigateur par l'événement `hello`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(SUlid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(SUlid::new())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Les compteurs d'une connexion, partagés entre le registre (qui compte les
/// pertes) et le flux (qui compte ce qu'il a réellement rendu au corps).
#[derive(Debug, Default)]
pub struct ConnectionStats {
    delivered: AtomicU64,
    dropped: AtomicU64,
}

impl ConnectionStats {
    pub fn record_delivered(&self) {
        self.delivered.fetch_add(1, Ordering::Relaxed);
    }

    fn record_dropped(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn delivered(&self) -> u64 {
        self.delivered.load(Ordering::Relaxed)
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

struct Connection {
    user_id: UserId,
    // Pris sur `AuthSession` à l'ouverture : la présence (644) et le routage
    // des notifications (645) le liront. Rien ne le lit encore.
    #[allow(dead_code)]
    coach_name: CoachName,
    sender: mpsc::Sender<RealtimeMessage>,
    stats: Arc<ConnectionStats>,
}

/// Ce que l'ouverture rend au flux : son identifiant, sa file, ses compteurs.
pub struct OpenedConnection {
    pub id: ConnectionId,
    pub receiver: mpsc::Receiver<RealtimeMessage>,
    pub stats: Arc<ConnectionStats>,
}

/// L'issue d'une livraison. Aucune ne bloque.
#[derive(Debug, PartialEq, Eq)]
pub enum Delivery {
    /// Mis en file : le flux le rendra au navigateur.
    Queued,
    /// File pleine : le message est perdu, et compté.
    Dropped,
    /// La connexion n'existe plus, ou son flux est déjà tombé.
    Gone,
}

#[derive(Default)]
pub struct ConnectionRegistry {
    connections: DashMap<ConnectionId, Connection>,
    by_user: DashMap<UserId, HashSet<ConnectionId>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre une connexion et rend la file que le flux consommera.
    pub fn open(&self, user_id: UserId, coach_name: CoachName) -> OpenedConnection {
        let id = ConnectionId::new();
        let (sender, receiver) = mpsc::channel(QUEUE_CAPACITY);
        let stats = Arc::new(ConnectionStats::default());
        let connection = Connection {
            user_id,
            coach_name,
            sender,
            stats: stats.clone(),
        };
        self.by_user.entry(user_id).or_default().insert(id);
        self.connections.insert(id, connection);
        OpenedConnection {
            id,
            receiver,
            stats,
        }
    }

    /// Retire une connexion — appelé par le `Drop` de la garde du flux, seul
    /// nettoyage du registre.
    pub fn close(&self, id: &ConnectionId) {
        let Some((_, connection)) = self.connections.remove(id) else {
            return;
        };
        let user_id = connection.user_id;
        if let Some(mut ids) = self.by_user.get_mut(&user_id) {
            ids.remove(id);
        }
        self.by_user.remove_if(&user_id, |_, ids| ids.is_empty());
    }

    /// Livre un message à une connexion, sans jamais attendre.
    pub fn deliver(&self, id: &ConnectionId, message: RealtimeMessage) -> Delivery {
        let Some(connection) = self.connections.get(id) else {
            return Delivery::Gone;
        };
        match connection.sender.try_send(message) {
            Ok(()) => Delivery::Queued,
            Err(TrySendError::Full(_)) => {
                connection.stats.record_dropped();
                tracing::warn!(conn = %id, "file pleine, message perdu");
                Delivery::Dropped
            }
            Err(TrySendError::Closed(_)) => Delivery::Gone,
        }
    }

    /// Connexions ouvertes, tous utilisateurs confondus.
    pub fn open_count(&self) -> usize {
        self.connections.len()
    }

    /// Connexions ouvertes par cet utilisateur — ses onglets.
    pub fn user_connection_count(&self, user_id: &UserId) -> usize {
        self.by_user.get(user_id).map(|ids| ids.len()).unwrap_or(0)
    }
}
