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
//! chaque BC pose sa propre condition devant `est_admin`, qui n'en sait rien.

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
}

/// La règle : admin de l'espace, ou admin de la compétition.
///
/// L'espace d'abord, la compétition ensuite, la première réponse positive
/// court-circuitant la seconde. Sans compétition — une page d'espace, un menu —
/// la seconde question n'est pas posée.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn est_admin(
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
    admins_d_espace: Vec<(String, String)>,
    admins_de_competition: Vec<(String, String)>,
    appels_espace: std::sync::atomic::AtomicUsize,
    appels_competition: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl FakeAdminAccess {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn admin_espace(mut self, coach: &CoachId, espace: &SpaceId) -> Self {
        self.admins_d_espace
            .push((coach.to_string(), espace.to_string()));
        self
    }

    pub fn admin_competition(mut self, coach: &CoachId, competition: &CompetitionId) -> Self {
        self.admins_de_competition
            .push((coach.to_string(), competition.to_string()));
        self
    }

    pub fn appels_espace(&self) -> usize {
        self.appels_espace.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn appels_competition(&self) -> usize {
        self.appels_competition
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
#[async_trait]
impl IAdminAccessPort for FakeAdminAccess {
    async fn is_space_admin(&self, user_id: &CoachId, space_id: &SpaceId) -> bool {
        self.appels_espace
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let cle = (user_id.to_string(), space_id.to_string());
        self.admins_d_espace.contains(&cle)
    }

    async fn is_competition_admin(
        &self,
        user_id: &CoachId,
        competition_id: &CompetitionId,
    ) -> bool {
        self.appels_competition
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let cle = (user_id.to_string(), competition_id.to_string());
        self.admins_de_competition.contains(&cle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::identity::ids::EntityId;

    fn ids() -> (CoachId, SpaceId, CompetitionId) {
        (EntityId::new(), EntityId::new(), EntityId::new())
    }

    async fn demande(
        port: &FakeAdminAccess,
        coach: &CoachId,
        espace: &SpaceId,
        competition: Option<&CompetitionId>,
    ) -> bool {
        est_admin(port, coach, espace, competition).await
    }

    #[tokio::test]
    async fn un_admin_d_espace_est_admin_avec_ou_sans_competition() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new().admin_espace(&coach, &espace);
        assert!(demande(&port, &coach, &espace, None).await);
        assert!(demande(&port, &coach, &espace, Some(&competition)).await);
    }

    /// L'espace suffit : la compétition n'est pas interrogée.
    #[tokio::test]
    async fn l_espace_court_circuite_la_competition() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new().admin_espace(&coach, &espace);
        assert!(demande(&port, &coach, &espace, Some(&competition)).await);
        assert_eq!(port.appels_competition(), 0);
    }

    #[tokio::test]
    async fn un_admin_de_competition_est_admin_de_sa_competition() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new().admin_competition(&coach, &competition);
        assert!(demande(&port, &coach, &espace, Some(&competition)).await);
    }

    /// Sans compétition, la seconde question n'est pas posée : un admin de
    /// compétition n'administre pas l'espace.
    #[tokio::test]
    async fn sans_competition_un_admin_de_competition_n_est_pas_admin() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new().admin_competition(&coach, &competition);
        assert!(!demande(&port, &coach, &espace, None).await);
        assert_eq!(
            port.appels_competition(),
            0,
            "la seconde question n'est pas posée"
        );
    }

    #[tokio::test]
    async fn l_admin_d_une_autre_competition_ne_l_est_pas_de_celle_ci() {
        let (coach, espace, competition) = ids();
        let autre: CompetitionId = EntityId::new();
        let port = FakeAdminAccess::new().admin_competition(&coach, &autre);
        assert!(!demande(&port, &coach, &espace, Some(&competition)).await);
    }

    #[tokio::test]
    async fn personne_n_est_admin_par_defaut() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new();
        assert!(!demande(&port, &coach, &espace, Some(&competition)).await);
    }
}
