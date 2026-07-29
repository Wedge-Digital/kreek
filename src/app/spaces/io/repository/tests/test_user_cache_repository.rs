use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::email::Email;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::{
    ISpaceUserCacheRepository, SpaceUserCacheRepositoryError,
};
use crate::app::spaces::domain::user::User;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;
use crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository;
use sqlx::PgPool;

fn make_user(name: &str, email: &str) -> User {
    User {
        id: CoachId::new(),
        name: CoachName::try_new(name).unwrap(),
        email: Email::try_new(email).unwrap(),
        icon: None,
    }
}

#[sqlx::test]
async fn add_user_inserts_user(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);
    let user = make_user("Bagouze", "coach@example.com");

    let result = repo.add_user(&user).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn add_user_duplicate_coach_name_returns_error(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);
    let user1 = make_user("Bagouze", "coach1@example.com");
    let user2 = make_user("Bagouze", "coach2@example.com");

    repo.add_user(&user1).await.unwrap();
    let result = repo.add_user(&user2).await;

    assert!(matches!(
        result,
        Err(SpaceUserCacheRepositoryError::UsernameNameAlreadyPresentInCache)
    ));
}

#[sqlx::test]
async fn add_user_duplicate_email_returns_error(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);
    let user1 = make_user("Bagouze", "coach@example.com");
    let user2 = make_user("Autre", "coach@example.com");

    repo.add_user(&user1).await.unwrap();
    let result = repo.add_user(&user2).await;

    // la contrainte UNIQUE sur email déclenche une erreur DB
    assert!(matches!(
        result,
        Err(SpaceUserCacheRepositoryError::Database(_))
    ));
}

#[sqlx::test]
async fn find_user_by_id_returns_correct_fields(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);
    let user = make_user("Bagouze", "coach@example.com");
    let id = user.id;

    repo.add_user(&user).await.unwrap();
    let found = repo.find_user_by_id(&id).await.unwrap();

    assert_eq!(found.id, id);
    assert_eq!(found.name.clone().into_inner(), "Bagouze");
    assert_eq!(found.email.value(), "coach@example.com");
    assert!(found.icon.is_none());
}

#[sqlx::test]
async fn find_user_by_id_returns_not_found(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);

    let result = repo.find_user_by_id(&CoachId::new()).await;

    assert!(matches!(
        result,
        Err(SpaceUserCacheRepositoryError::UserNotFoundInCache)
    ));
}

#[sqlx::test]
async fn find_all_users_returns_empty_on_fresh_db(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);

    let result = repo.find_all_users().await.unwrap();

    assert!(result.is_empty());
}

#[sqlx::test]
async fn find_all_users_returns_all_inserted(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);
    let user1 = make_user("Alice", "alice@example.com");
    let user2 = make_user("Bob", "bob@example.com");

    repo.add_user(&user1).await.unwrap();
    repo.add_user(&user2).await.unwrap();

    let all = repo.find_all_users().await.unwrap();

    assert_eq!(all.len(), 2);
}

#[sqlx::test]
async fn find_all_users_are_ordered_by_coach_name(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);

    repo.add_user(&make_user("Zorro", "zorro@example.com"))
        .await
        .unwrap();
    repo.add_user(&make_user("Alice", "alice@example.com"))
        .await
        .unwrap();
    repo.add_user(&make_user("Martin", "martin@example.com"))
        .await
        .unwrap();

    let all = repo.find_all_users().await.unwrap();

    let names: Vec<_> = all.iter().map(|u| u.name.clone().into_inner()).collect();
    assert_eq!(names, vec!["Alice", "Martin", "Zorro"]);
}

// ── list_members_for_space ────────────────────────────────────────────────────

#[sqlx::test]
async fn list_members_for_space_returns_empty_when_no_members(pool: PgPool) {
    let repo = SpaceUserCacheRepository::new(pool);

    let result = repo.list_members_for_space(&SpaceId::new()).await.unwrap();

    assert!(result.is_empty());
}

#[sqlx::test]
async fn list_members_for_space_returns_only_members_of_given_space(pool: PgPool) {
    let cache_repo = SpaceUserCacheRepository::new(pool.clone());
    let space_repo = SpaceRepository::new(pool);

    let space_a = SpaceId::new();
    let space_b = SpaceId::new();
    let alice = make_user("Alice", "alice@example.com");
    let bob = make_user("Bob", "bob@example.com");

    cache_repo.add_user(&alice).await.unwrap();
    cache_repo.add_user(&bob).await.unwrap();
    space_repo
        .add_member(&space_a, &alice.id, &SpaceProfile::SpaceUser)
        .await
        .unwrap();
    space_repo
        .add_member(&space_b, &bob.id, &SpaceProfile::SpaceUser)
        .await
        .unwrap();

    let members = cache_repo.list_members_for_space(&space_a).await.unwrap();

    assert_eq!(members.len(), 1);
    assert_eq!(members[0].name.clone().into_inner(), "Alice");
}

#[sqlx::test]
async fn list_members_for_space_returns_all_members_of_given_space(pool: PgPool) {
    let cache_repo = SpaceUserCacheRepository::new(pool.clone());
    let space_repo = SpaceRepository::new(pool);

    let space_id = SpaceId::new();
    let alice = make_user("Alice", "alice@example.com");
    let bob = make_user("Bob", "bob@example.com");

    cache_repo.add_user(&alice).await.unwrap();
    cache_repo.add_user(&bob).await.unwrap();
    space_repo
        .add_member(&space_id, &alice.id, &SpaceProfile::SpaceUser)
        .await
        .unwrap();
    space_repo
        .add_member(&space_id, &bob.id, &SpaceProfile::SpaceUser)
        .await
        .unwrap();

    let members = cache_repo.list_members_for_space(&space_id).await.unwrap();

    assert_eq!(members.len(), 2);
}

#[sqlx::test]
async fn list_members_for_space_ordered_by_coach_name(pool: PgPool) {
    let cache_repo = SpaceUserCacheRepository::new(pool.clone());
    let space_repo = SpaceRepository::new(pool);

    let space_id = SpaceId::new();
    let zorro = make_user("Zorro", "zorro@example.com");
    let alice = make_user("Alice", "alice@example.com");
    let martin = make_user("Martin", "martin@example.com");

    cache_repo.add_user(&zorro).await.unwrap();
    cache_repo.add_user(&alice).await.unwrap();
    cache_repo.add_user(&martin).await.unwrap();
    space_repo
        .add_member(&space_id, &zorro.id, &SpaceProfile::SpaceUser)
        .await
        .unwrap();
    space_repo
        .add_member(&space_id, &alice.id, &SpaceProfile::SpaceUser)
        .await
        .unwrap();
    space_repo
        .add_member(&space_id, &martin.id, &SpaceProfile::SpaceUser)
        .await
        .unwrap();

    let members = cache_repo.list_members_for_space(&space_id).await.unwrap();

    let names: Vec<_> = members
        .iter()
        .map(|u| u.name.clone().into_inner())
        .collect();
    assert_eq!(names, vec!["Alice", "Martin", "Zorro"]);
}
