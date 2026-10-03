//! Qui est admin — un seul service pour tout kreek (carte 570).
//!
//! # Pourquoi dans le noyau partagé
//!
//! « Admin d'espace ou de compétition » s'écrivait en six copies, dans six BCs,
//! chacune avec son port et son adapter — et elles ne disaient pas toutes la
//! même chose. La règle vit désormais ici, une fois ; chaque BC reçoit le même
//! port et pose la même question.
//!
//! C'est un **contrat partagé**, pas un import entre BCs : le noyau joue déjà ce
//! rôle (`SpaceProfile`, `identity::charset`). La règle « Adapters inter-BCs »
//! du `CLAUDE.md` reste la règle par défaut ; celle-ci en est l'exception
//! assumée, écrite dans le même fichier.
//!
//! # Pourquoi `bloodbowl` et non `identity`
//!
//! La règle parle de compétition, que `auth` et `spaces` — extractibles — ne
//! connaissent pas. Eux gardent `SpacePermissions::is_admin()`, qui ne répond
//! que pour l'espace.
//!
//! # Ce que le service ne décide pas
//!
//! **Le propriétaire.** Inclus pour l'effectif, exclu pour la customisation :
//! chaque BC pose sa propre condition devant `is_admin`, qui n'en sait rien.

use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use async_trait::async_trait;

/// Ce que la règle a besoin de savoir. Implémenté **une fois**, dans
/// `infrastructure/admin_access/` ; remplacé en test par `FakeAdminAccess`.
///
/// **L'identifiant seul.** Les admins de compétition ne se reconnaissent que par
/// identifiant ; le compte exploitant, qui a tous les droits, est résolu par
/// l'adapter lui-même. Aucun appelant n'a donc à transporter de nom.
///
/// Une erreur de lecture rend `false` : un contrôle d'accès échoue fermé.
#[async_trait]
pub trait IAdminAccessPort: Send + Sync {
    async fn is_space_admin(&self, user_id: &CoachId, space_id: &SpaceId) -> bool;

    async fn is_competition_admin(&self, user_id: &CoachId, competition_id: &CompetitionId)
        -> bool;

    /// Admin d'**au moins un** espace — le compte exploitant compris (carte
    /// 573). La question d'une route qui n'a pas d'espace dans son chemin : la
    /// création d'un compte coach, ouverte à ceux qui voient le panneau.
    async fn is_admin_of_any_space(&self, user_id: &CoachId) -> bool;
}

/// La règle : admin de l'espace, ou admin de la compétition.
///
/// L'espace d'abord, la compétition ensuite, la première réponse positive
/// court-circuitant la seconde. Sans compétition — une page d'espace, un menu —
/// la seconde question n'est pas posée.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn is_admin(
    port: &dyn IAdminAccessPort,
    user_id: &CoachId,
    space_id: &SpaceId,
    competition_id: Option<&CompetitionId>,
) -> bool {
    if port.is_space_admin(user_id, space_id).await {
        return true;
    }
    match competition_id {
        Some(competition) => port.is_competition_admin(user_id, competition).await,
        None => false,
    }
}

/// La doublure partagée : elle refuse tout ce qu'on ne lui a pas dit.
///
/// Chaque BC en écrivait une ; elles disparaissent avec leurs ports.
///
/// **Elle compte ses appels** : c'est ce qui permet de vérifier qu'un
/// propriétaire n'en déclenche aucun, ou qu'une équipe sans compétition ne pose
/// pas la seconde question.
#[cfg(test)]
#[derive(Default)]
pub struct FakeAdminAccess {
    space_admins: Vec<(String, String)>,
    competition_admins: Vec<(String, String)>,
    space_calls: std::sync::atomic::AtomicUsize,
    competition_calls: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl FakeAdminAccess {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn space_admin(mut self, coach: &CoachId, space: &SpaceId) -> Self {
        self.space_admins
            .push((coach.to_string(), space.to_string()));
        self
    }

    pub fn competition_admin(mut self, coach: &CoachId, competition: &CompetitionId) -> Self {
        self.competition_admins
            .push((coach.to_string(), competition.to_string()));
        self
    }

    pub fn space_calls(&self) -> usize {
        self.space_calls.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn competition_calls(&self) -> usize {
        self.competition_calls
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
#[async_trait]
impl IAdminAccessPort for FakeAdminAccess {
    async fn is_space_admin(&self, user_id: &CoachId, space_id: &SpaceId) -> bool {
        self.space_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let cle = (user_id.to_string(), space_id.to_string());
        self.space_admins.contains(&cle)
    }

    async fn is_competition_admin(
        &self,
        user_id: &CoachId,
        competition_id: &CompetitionId,
    ) -> bool {
        self.competition_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let cle = (user_id.to_string(), competition_id.to_string());
        self.competition_admins.contains(&cle)
    }

    async fn is_admin_of_any_space(&self, user_id: &CoachId) -> bool {
        let coach = user_id.to_string();
        self.space_admins.iter().any(|(c, _)| *c == coach)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::identity::ids::EntityId;

    fn ids() -> (CoachId, SpaceId, CompetitionId) {
        (EntityId::new(), EntityId::new(), EntityId::new())
    }

    async fn ask(
        port: &FakeAdminAccess,
        coach: &CoachId,
        space: &SpaceId,
        competition: Option<&CompetitionId>,
    ) -> bool {
        is_admin(port, coach, space, competition).await
    }

    #[tokio::test]
    async fn space_admin_is_admin_with_or_without_competition() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new().space_admin(&coach, &space);
        assert!(ask(&port, &coach, &space, None).await);
        assert!(ask(&port, &coach, &space, Some(&competition)).await);
    }

    /// L'espace suffit : la compétition n'est pas interrogée.
    #[tokio::test]
    async fn space_short_circuits_competition() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new().space_admin(&coach, &space);
        assert!(ask(&port, &coach, &space, Some(&competition)).await);
        assert_eq!(port.competition_calls(), 0);
    }

    #[tokio::test]
    async fn competition_admin_is_admin_of_their_competition() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new().competition_admin(&coach, &competition);
        assert!(ask(&port, &coach, &space, Some(&competition)).await);
    }

    /// Sans compétition, la seconde question n'est pas posée : un admin de
    /// compétition n'administre pas l'espace.
    #[tokio::test]
    async fn without_competition_a_competition_admin_is_not_admin() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new().competition_admin(&coach, &competition);
        assert!(!ask(&port, &coach, &space, None).await);
        assert_eq!(
            port.competition_calls(),
            0,
            "la seconde question n'est pas posée"
        );
    }

    #[tokio::test]
    async fn admin_of_another_competition_is_not_admin_here() {
        let (coach, space, competition) = ids();
        let other: CompetitionId = EntityId::new();
        let port = FakeAdminAccess::new().competition_admin(&coach, &other);
        assert!(!ask(&port, &coach, &space, Some(&competition)).await);
    }

    #[tokio::test]
    async fn nobody_is_admin_by_default() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new();
        assert!(!ask(&port, &coach, &space, Some(&competition)).await);
    }
}
