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
    ISpaceRepository, SpaceMemberRow, SpaceRepositoryError, SpaceSummary,
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
}

impl FakeSpaceRepo {
    pub fn avec(space: Space) -> Self {
        Self {
            space: Some(space),
            profils_ecrits: AtomicUsize::new(0),
            membres_supprimes: AtomicUsize::new(0),
        }
    }

    pub fn vide() -> Self {
        Self {
            space: None,
            profils_ecrits: AtomicUsize::new(0),
            membres_supprimes: AtomicUsize::new(0),
        }
    }

    pub fn ecritures(&self) -> usize {
        self.profils_ecrits.load(Ordering::SeqCst) + self.membres_supprimes.load(Ordering::SeqCst)
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
}
