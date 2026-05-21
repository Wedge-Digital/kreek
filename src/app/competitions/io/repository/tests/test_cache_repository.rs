use sqlx::PgPool;
use crate::app::competitions::domain::cache_repository_port::{CachedSpace, CachedUser, ICompetitionsCacheRepository};
use crate::app::competitions::io::repository::cache_repository::CompetitionsCacheRepository;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::email::Email;
use crate::app::shared_kernel::space_name::SpaceName;

fn make_user(id: CoachId, name: &str, email: &str) -> CachedUser {
    CachedUser {
        id,
        coach_name: CoachName::try_new(name).unwrap(),
        coach_icon: None,
        email:      Email::try_new(email).unwrap(),
    }
}

fn make_space(id: SpaceId, name: &str) -> CachedSpace {
    CachedSpace {
        space_id: id,
        space_name: SpaceName::try_new(name).expect("REASON"),
        space_logo: CloudinaryImage::try_new("https://res.cloudinary.com/demo/image/upload/sample.jpg").unwrap()
    }
}

// --- user ---

#[sqlx::test]
async fn add_user_inserts_user(pool: PgPool) {
    let repo = CompetitionsCacheRepository::new(pool);

    let result = repo.add_user(&make_user(CoachId::new(), "Bagouze", "bagouze@example.com")).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn add_user_same_id_is_idempotent(pool: PgPool) {
    let repo = CompetitionsCacheRepository::new(pool);
    let user = make_user(CoachId::new(), "Bagouze", "bagouze@example.com");

    repo.add_user(&user).await.unwrap();
    let result = repo.add_user(&user).await;

    // ON CONFLICT (id) DO NOTHING
    assert!(result.is_ok());
}

#[sqlx::test]
async fn remove_user_deletes_existing_user(pool: PgPool) {
    let repo    = CompetitionsCacheRepository::new(pool);
    let user_id = CoachId::new();

    repo.add_user(&make_user(user_id, "Bagouze", "bagouze@example.com")).await.unwrap();
    let result = repo.remove_user(&user_id).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn remove_user_on_absent_id_is_idempotent(pool: PgPool) {
    let repo = CompetitionsCacheRepository::new(pool);

    let result = repo.remove_user(&CoachId::new()).await;

    assert!(result.is_ok());
}

// --- space ---

#[sqlx::test]
async fn add_space_inserts_space(pool: PgPool) {
    let repo = CompetitionsCacheRepository::new(pool);

    let result = repo.add_space(&make_space(SpaceId::new(), "LigueAlpha")).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn remove_space_deletes_existing_space(pool: PgPool) {
    let repo     = CompetitionsCacheRepository::new(pool);
    let space_id = SpaceId::new();

    repo.add_space(&make_space(space_id, "LigueAlpha")).await.unwrap();
    let result = repo.remove_space(&space_id).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn remove_space_on_absent_id_is_idempotent(pool: PgPool) {
    let repo = CompetitionsCacheRepository::new(pool);

    let result = repo.remove_space(&SpaceId::new()).await;

    assert!(result.is_ok());
}

// --- subscribe / unsubscribe ---

#[sqlx::test]
async fn subscribe_inserts_user_space_link(pool: PgPool) {
    let repo     = CompetitionsCacheRepository::new(pool);
    let user_id  = CoachId::new();
    let space_id = SpaceId::new();

    repo.add_user(&make_user(user_id, "Bagouze", "bagouze@example.com")).await.unwrap();
    repo.add_space(&make_space(space_id, "LigueAlpha")).await.unwrap();

    let result = repo.subscribe(&user_id, &space_id, &SpaceProfile::SpaceUser).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn subscribe_upserts_profile_on_conflict(pool: PgPool) {
    let repo     = CompetitionsCacheRepository::new(pool);
    let user_id  = CoachId::new();
    let space_id = SpaceId::new();

    repo.add_user(&make_user(user_id, "Bagouze", "bagouze@example.com")).await.unwrap();
    repo.add_space(&make_space(space_id, "LigueAlpha")).await.unwrap();
    repo.subscribe(&user_id, &space_id, &SpaceProfile::SpaceUser).await.unwrap();

    // ON CONFLICT DO UPDATE SET profile — doit réussir et mettre à jour le profil
    let result = repo.subscribe(&user_id, &space_id, &SpaceProfile::SpaceAdmin).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn unsubscribe_removes_link(pool: PgPool) {
    let repo     = CompetitionsCacheRepository::new(pool);
    let user_id  = CoachId::new();
    let space_id = SpaceId::new();

    repo.add_user(&make_user(user_id, "Bagouze", "bagouze@example.com")).await.unwrap();
    repo.add_space(&make_space(space_id, "LigueAlpha")).await.unwrap();
    repo.subscribe(&user_id, &space_id, &SpaceProfile::SpaceUser).await.unwrap();

    let result = repo.unsubscribe(&user_id, &space_id).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn unsubscribe_on_absent_link_is_idempotent(pool: PgPool) {
    let repo = CompetitionsCacheRepository::new(pool);

    let result = repo.unsubscribe(&CoachId::new(), &SpaceId::new()).await;

    assert!(result.is_ok());
}