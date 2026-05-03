use sqlx::PgPool;
use crate::app::auth::domain::coach_name::CoachName;
use crate::app::auth::domain::email::Email;
use crate::app::auth::io::repository::user_repository::UserRepository;
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::common_types::UserId;
use crate::app::shared_kernel::user::User;

fn make_user(coach_name: &str, email: &str) -> User {
    User::new(
        UserId::new(),
        CoachName::try_new(coach_name).unwrap(),
        Email::try_new(email).unwrap(),
        "hash_fictif".into(),
    )
}

// --- create ---

#[sqlx::test]
async fn create_persiste_un_utilisateur(pool: PgPool) {
    let repo = UserRepository::new(pool);

    let result = repo.create(&make_user("Bagouze", "bagouze@example.com")).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn create_renvoie_coach_name_already_taken(pool: PgPool) {
    let repo = UserRepository::new(pool);
    repo.create(&make_user("Bagouze", "premier@example.com")).await.unwrap();

    let result = repo.create(&make_user("Bagouze", "second@example.com")).await;

    assert!(matches!(result, Err(RepositoryError::CoachNameAlreadyTaken)));
}

#[sqlx::test]
async fn create_renvoie_email_already_taken(pool: PgPool) {
    let repo = UserRepository::new(pool);
    repo.create(&make_user("PremierCoach", "partage@example.com")).await.unwrap();

    let result = repo.create(&make_user("SecondCoach", "partage@example.com")).await;

    assert!(matches!(result, Err(RepositoryError::EmailAlreadyTaken)));
}

// --- find_by_coach_name ---

#[sqlx::test]
async fn find_by_coach_name_renvoie_none_si_absent(pool: PgPool) {
    let repo = UserRepository::new(pool);

    let result = repo.find_by_coach_name("Inconnu").await.unwrap();

    assert!(result.is_none());
}

#[sqlx::test]
async fn find_by_coach_name_renvoie_le_bon_utilisateur(pool: PgPool) {
    let repo = UserRepository::new(pool);
    let user = make_user("Bagouze", "bagouze@example.com");
    repo.create(&user).await.unwrap();

    let found = repo.find_by_coach_name("Bagouze").await.unwrap().unwrap();

    assert_eq!(found.coach_name.into_inner(), "Bagouze");
    assert_eq!(found.email.value(), "bagouze@example.com");
    assert_eq!(found.password_hash, "hash_fictif");
}

#[sqlx::test]
async fn find_by_coach_name_preserves_id(pool: PgPool) {
    let repo = UserRepository::new(pool);
    let user = make_user("Bagouze", "bagouze@example.com");
    let id_original = user.id.to_string();
    repo.create(&user).await.unwrap();

    let found = repo.find_by_coach_name("Bagouze").await.unwrap().unwrap();

    assert_eq!(found.id.to_string(), id_original);
}

#[sqlx::test]
async fn find_by_coach_name_est_sensible_a_la_casse(pool: PgPool) {
    let repo = UserRepository::new(pool);
    repo.create(&make_user("Bagouze", "bagouze@example.com")).await.unwrap();

    let result = repo.find_by_coach_name("bagouze").await.unwrap();

    assert!(result.is_none());
}