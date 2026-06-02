use crate::app::auth::domain::reset_token::Token;
use crate::app::auth::io::repository::reset_token_repository::{
    IResetTokenRepository, ResetTokenRepository,
};
use crate::app::shared_kernel::coach_name::CoachName;
use sqlx::PgPool;

fn make_token() -> Token {
    Token::new()
}
fn make_coach_name(s: &str) -> CoachName {
    CoachName::try_new(s).unwrap()
}

#[sqlx::test]
async fn create_inserts_token(pool: PgPool) {
    let repo = ResetTokenRepository::new(pool);

    let result = repo
        .create(&make_token(), &make_coach_name("Bagouze"))
        .await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn find_by_token_returns_token_with_correct_fields(pool: PgPool) {
    let repo = ResetTokenRepository::new(pool);
    let token = make_token();
    let coach_name = make_coach_name("Bagouze");

    repo.create(&token, &coach_name).await.unwrap();
    let found = repo
        .find_by_token(&token.to_string())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.token.to_string(), token.to_string());
    assert_eq!(found.coach_name.clone().into_inner(), "Bagouze");
}

#[sqlx::test]
async fn find_by_token_returns_none_for_unknown_token(pool: PgPool) {
    let repo = ResetTokenRepository::new(pool);

    let result = repo.find_by_token("nonexistent-token").await.unwrap();

    assert!(result.is_none());
}

#[sqlx::test]
async fn delete_by_token_removes_it(pool: PgPool) {
    let repo = ResetTokenRepository::new(pool);
    let token = make_token();

    repo.create(&token, &make_coach_name("Bagouze"))
        .await
        .unwrap();
    repo.delete_by_token(&token.to_string()).await.unwrap();

    let found = repo.find_by_token(&token.to_string()).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test]
async fn delete_by_token_on_absent_token_is_idempotent(pool: PgPool) {
    let repo = ResetTokenRepository::new(pool);

    let result = repo.delete_by_token("nonexistent-token").await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn create_same_token_twice_upserts(pool: PgPool) {
    let repo = ResetTokenRepository::new(pool);
    let token = make_token();
    let coach_name = make_coach_name("Bagouze");

    repo.create(&token, &coach_name).await.unwrap();
    let result = repo.create(&token, &coach_name).await;

    // ON CONFLICT (token) DO UPDATE SET created_at — doit réussir
    assert!(result.is_ok());
}
