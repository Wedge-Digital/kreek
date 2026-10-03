use crate::app::auth::domain::user::User;
use async_trait::async_trait;
use std::fmt;

#[derive(Debug)]
pub enum RepositoryError {
    CoachNameAlreadyTaken,
    EmailAlreadyTaken,
    Database(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepositoryError::CoachNameAlreadyTaken => write!(f, "Ce nom de coach est déjà utilisé"),
            RepositoryError::EmailAlreadyTaken => {
                write!(f, "Cette adresse email est déjà utilisée")
            }
            RepositoryError::Database(msg) => write!(f, "Erreur base de données : {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

#[async_trait]
pub trait IUserRepository: Send + Sync {
    async fn create(&self, user: &User) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, RepositoryError>;
    async fn find_by_legacy_id(&self, legacy_id: i32) -> Result<Option<User>, RepositoryError>;
    async fn find_by_coach_name(&self, coach_name: &str) -> Result<Option<User>, RepositoryError>;
    async fn update_password_hash(
        &self,
        coach_name: &str,
        new_hash: &str,
    ) -> Result<(), RepositoryError>;
}

/// Qui peut créer un compte coach depuis le widget (carte 573).
///
/// **Décidé par l'hôte, pas par `auth`** : la règle — admin d'au moins un
/// espace, ou le compte exploitant — parle d'espaces, et ce BC extractible n'en
/// connaît aucun. Il décrit son besoin ; `infrastructure/auth/` y répond, et
/// `main.rs` l'injecte dans `AuthContext`, sur le modèle d'`ISpacesHostLayout`.
#[async_trait]
pub trait IAccountCreationPolicy: Send + Sync {
    async fn may_create_accounts(
        &self,
        user_id: &crate::app::shared_kernel::identity::ids::UserId,
    ) -> bool;
}
