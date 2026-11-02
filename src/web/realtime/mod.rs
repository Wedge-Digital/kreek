//! Le canal temps réel du serveur vers le navigateur (épic E23).
//!
//! Un flux SSE par onglet authentifié, et le registre qui sait lesquels sont
//! ouverts. La carte 642 n'y fait passer que l'accueil et le battement de
//! cœur ; la présence (644), les notifications (645) et le direct (646) s'y
//! branchent.

pub mod message;
pub mod registry;
pub mod stream_controller;

// Carte 645 : les notifications de match.
pub mod notification_router;
pub mod toast;

#[cfg(test)]
mod tests;
