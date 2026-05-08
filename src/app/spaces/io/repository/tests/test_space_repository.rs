#[sqlx::test]
async fn create_persiste_un_utilisateur(pool: PgPool) {
    let repo = SpaceRepository::new(pool);

    let result = repo.create(&make_user("Bagouze", "bagouze@example.com")).await;

    assert!(result.is_ok());
}
