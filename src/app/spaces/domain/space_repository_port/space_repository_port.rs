use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
use crate::app::spaces::domain::space::Space;
use async_trait::async_trait;
use std::fmt;

/// Une ligne de la liste des membres, pour l'administration d'un espace.
///
/// DTO de **lecture** : primitives assumées, aucun invariant à protéger. Il ne
/// remonte jamais jusqu'à un gabarit — les VMs sont bâtis à partir de lui.
pub struct SpaceMemberRow {
    pub coach_id: String,
    pub coach_name: String,
    pub email: String,
    pub icon: Option<String>,
    /// « SpaceAdmin » ou « SpaceUser ». Converti par le consommateur.
    pub profile: String,
}

/// Un coach de l'annuaire de la plateforme, vu depuis un espace donné.
///
/// DTO de **lecture** : primitives assumées. `est_membre` vient d'une jointure
/// externe — les membres sont **rendus, pas exclus**, pour ne pas laisser croire
/// qu'un coach n'existe pas alors qu'il est déjà là.
pub struct CandidateRow {
    pub coach_id: String,
    pub coach_name: String,
    pub email: String,
    pub icon: Option<String>,
    pub est_membre: bool,
}

pub struct SpaceSummary {
    pub id: String,
    pub name: String,
    pub logo: CloudinaryImage,
}

#[derive(Debug)]
pub enum SpaceRepositoryError {
    SpaceNameAlreadyTaken,
    CoachAlreadyMember,
    Database(String),
}

impl fmt::Display for SpaceRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpaceRepositoryError::SpaceNameAlreadyTaken => {
                write!(f, "Ce nom d'espace est déjà utilisé")
            }
            SpaceRepositoryError::CoachAlreadyMember => {
                write!(f, "Ce coach est déjà membre de cet espace")
            }
            SpaceRepositoryError::Database(msg) => write!(f, "Erreur base de données : {}", msg),
        }
    }
}

impl std::error::Error for SpaceRepositoryError {}

#[async_trait]
pub trait ISpaceRepository: Send + Sync {
    async fn save(&self, space: &Space) -> Result<(), SpaceRepositoryError>;
    async fn add_member(
        &self,
        space_id: &SpaceId,
        coach_id: &CoachId,
        profile: &SpaceProfile,
    ) -> Result<(), SpaceRepositoryError>;
    async fn join_spaces(
        &self,
        space_ids: &[SpaceId],
        coach_id: &CoachId,
    ) -> Result<(), SpaceRepositoryError>;
    async fn find_by_id(&self, id: &SpaceId) -> Result<Option<Space>, SpaceRepositoryError>;
    async fn find_by_coach_id(
        &self,
        coach_id: &CoachId,
    ) -> Result<Vec<SpaceSummary>, SpaceRepositoryError>;
    async fn find_member_profile(
        &self,
        coach_id: &CoachId,
        space_id: &SpaceId,
    ) -> Result<Option<SpaceProfile>, SpaceRepositoryError>;
    async fn find_all(&self) -> Result<Vec<SpaceSummary>, SpaceRepositoryError>;

    /// Les membres d'un espace **avec leur profil**.
    ///
    /// Distincte de `list_members_for_space` du cache, qui rend des `User` sans
    /// profil : l'élargir ferait porter une colonne inutile à son appelant
    /// actuel, le sélecteur de coachs.
    async fn list_members_with_profile(
        &self,
        space_id: &SpaceId,
    ) -> Result<Vec<SpaceMemberRow>, SpaceRepositoryError>;

    async fn update_member_profile(
        &self,
        space_id: &SpaceId,
        coach_id: &CoachId,
        profile: &SpaceProfile,
    ) -> Result<(), SpaceRepositoryError>;

    async fn delete_member(
        &self,
        space_id: &SpaceId,
        coach_id: &CoachId,
    ) -> Result<(), SpaceRepositoryError>;

    /// Cherche des coachs dans l'annuaire de la plateforme, en marquant ceux
    /// qui sont déjà membres de l'espace.
    ///
    /// `limite` est décidée par l'appelant côté serveur, jamais reçue du client.
    async fn search_platform_coaches(
        &self,
        space_id: &SpaceId,
        q: &str,
        limite: i64,
    ) -> Result<Vec<CandidateRow>, SpaceRepositoryError>;
}
