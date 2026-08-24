//! Doublure en mémoire du dépôt de `spaces`, pour les use cases d'appartenance.
//!
//! Elle n'imite que ce que les tests observent : l'espace chargé, et la **trace
//! des écritures**.
//!
//! Cette trace n'est pas un agrément. Sur les chemins de refus, il ne suffit pas
//! de vérifier le type d'erreur : une implémentation qui écrirait d'abord et
//! validerait ensuite rendrait la même erreur, en ayant modifié la base. Compter
//! les appels est la seule façon de distinguer les deux.

use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::spaces::domain::coach::Coach;
use crate::app::spaces::domain::space::Space;
use crate::app::spaces::domain::space_repository_port::space_repository_port::{
    CandidateRow, ISpaceRepository, SpaceMemberRow, SpaceRepositoryError, SpaceSummary,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

const LOGO: &str = "https://res.cloudinary.com/demo/image/upload/space.jpg";

pub fn coach(nom: &str, profil: SpaceProfile) -> Coach {
    Coach::new(
        CoachId::new(),
        CoachName::try_new(nom).unwrap(),
        profil,
        None,
    )
}

pub fn espace(id: SpaceId, coaches: Vec<Coach>) -> Space {
    Space::new(
        id,
        SpaceName::try_new("Tribu Celtique").unwrap(),
        CloudinaryImage::try_new(LOGO).unwrap(),
        coaches,
    )
}

pub struct FakeSpaceRepo {
    /// `None` fait rendre `EspaceInconnu` au use case.
    pub space: Option<Space>,
    pub profils_ecrits: AtomicUsize,
    pub membres_supprimes: AtomicUsize,
    pub membres_ajoutes: AtomicUsize,
}

impl FakeSpaceRepo {
    pub fn avec(space: Space) -> Self {
        Self {
            space: Some(space),
            profils_ecrits: AtomicUsize::new(0),
            membres_supprimes: AtomicUsize::new(0),
            membres_ajoutes: AtomicUsize::new(0),
        }
    }

    pub fn vide() -> Self {
        Self {
            space: None,
            profils_ecrits: AtomicUsize::new(0),
            membres_supprimes: AtomicUsize::new(0),
            membres_ajoutes: AtomicUsize::new(0),
        }
    }

    pub fn ecritures(&self) -> usize {
        self.profils_ecrits.load(Ordering::SeqCst)
            + self.membres_supprimes.load(Ordering::SeqCst)
            + self.membres_ajoutes.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ISpaceRepository for FakeSpaceRepo {
    async fn save(&self, _: &Space) -> Result<(), SpaceRepositoryError> {
        Ok(())
    }
    async fn add_member(
        &self,
        _: &SpaceId,
        _: &CoachId,
        _: &SpaceProfile,
    ) -> Result<(), SpaceRepositoryError> {
        self.membres_ajoutes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn join_spaces(&self, _: &[SpaceId], _: &CoachId) -> Result<(), SpaceRepositoryError> {
        Ok(())
    }
    async fn find_by_id(&self, _: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError> {
        Ok(self.space.as_ref().map(|s| {
            Space::new(
                *s.id(),
                s.name().clone(),
                s.logo().clone(),
                s.coaches().to_vec(),
            )
        }))
    }
    async fn find_by_coach_id(
        &self,
        _: &CoachId,
    ) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
        Ok(vec![])
    }
    async fn find_member_profile(
        &self,
        _: &CoachId,
        _: &SpaceId,
    ) -> Result<Option<SpaceProfile>, SpaceRepositoryError> {
        Ok(None)
    }
    async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError> {
        Ok(vec![])
    }
    async fn list_members_with_profile(
        &self,
        _: &SpaceId,
    ) -> Result<Vec<SpaceMemberRow>, SpaceRepositoryError> {
        Ok(vec![])
    }
    async fn update_member_profile(
        &self,
        _: &SpaceId,
        _: &CoachId,
        _: &SpaceProfile,
    ) -> Result<(), SpaceRepositoryError> {
        self.profils_ecrits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn delete_member(&self, _: &SpaceId, _: &CoachId) -> Result<(), SpaceRepositoryError> {
        self.membres_supprimes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn search_platform_coaches(
        &self,
        _: &SpaceId,
        _: &str,
        _: i64,
    ) -> Result<Vec<CandidateRow>, SpaceRepositoryError> {
        Ok(vec![])
    }
}

// ── Le cache d'utilisateurs, et le service d'email ───────────────────────────

use crate::app::shared_kernel::identity::email::Email;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::{
    ISpaceUserCacheRepository, SpaceUserCacheRepositoryError,
};
use crate::app::spaces::domain::user::User;
use crate::common::services::email::{EmailError, IEmailService};

pub struct FakeUserCache {
    pub users: Vec<User>,
}

impl FakeUserCache {
    pub fn avec(users: Vec<User>) -> Self {
        Self { users }
    }
}

pub fn user(id: CoachId, nom: &str) -> User {
    User {
        id,
        name: CoachName::try_new(nom).unwrap(),
        email: Email::try_new(&format!("{nom}@bb.club")).unwrap(),
        icon: None,
    }
}

#[async_trait]
impl ISpaceUserCacheRepository for FakeUserCache {
    async fn add_user(&self, _: &User) -> Result<(), SpaceUserCacheRepositoryError> {
        Ok(())
    }
    async fn find_user_by_id(&self, id: &CoachId) -> Result<User, SpaceUserCacheRepositoryError> {
        self.users
            .iter()
            .find(|u| &u.id == id)
            .cloned()
            .ok_or(SpaceUserCacheRepositoryError::UserNotFoundInCache)
    }
    async fn find_all_users(&self) -> Result<Vec<User>, SpaceUserCacheRepositoryError> {
        Ok(self.users.clone())
    }
    async fn list_members_for_space(
        &self,
        _: &SpaceId,
    ) -> Result<Vec<User>, SpaceUserCacheRepositoryError> {
        Ok(vec![])
    }
}

/// Compte les envois, et sait échouer sur commande.
///
/// L'échec est la moitié utile : il sert au seul test qui vérifie que la
/// courtoisie ne gouverne pas l'appartenance.
pub struct FakeEmail {
    pub envois: AtomicUsize,
    pub echoue: bool,
}

impl FakeEmail {
    pub fn qui_marche() -> Self {
        Self {
            envois: AtomicUsize::new(0),
            echoue: false,
        }
    }
    pub fn en_panne() -> Self {
        Self {
            envois: AtomicUsize::new(0),
            echoue: true,
        }
    }
    pub fn envoyes(&self) -> usize {
        self.envois.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl IEmailService for FakeEmail {
    async fn send(&self, _: Vec<String>, _: String, _: String) -> Result<(), EmailError> {
        self.envois.fetch_add(1, Ordering::SeqCst);
        if self.echoue {
            Err(EmailError::Network("panne simulée".into()))
        } else {
            Ok(())
        }
    }
}
