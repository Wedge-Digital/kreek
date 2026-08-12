use crate::app::auth::domain::user::User;
use crate::app::auth::io::repository::user_repository::UserRepository;
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::email::Email;
use crate::app::shared_kernel::identity::ids::UserId;
use sqlx::PgPool;

fn make_user(coach_name: &str, email: &str) -> User {
    User::new(
        UserId::new(),
        CoachName::try_new(coach_name).unwrap(),
        // Option::from(CoachIcon::try_new("https://res.cloudinary.com/bloodbowlclub-com/image/upload/v1650731881/avatars/dxhokrr5hets7ncggrrk.png").unwrap()),
        Option::from(None),
        Email::try_new(email).unwrap(),
        "hash_fictif".into(),
    )
}

// --- create ---

#[sqlx::test]
async fn create_persiste_un_utilisateur(pool: PgPool) {
    let repo = UserRepository::new(pool);

    let result = repo
        .create(&make_user("Bagouze", "bagouze@example.com"))
        .await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn create_renvoie_coach_name_already_taken(pool: PgPool) {
    let repo = UserRepository::new(pool);
    repo.create(&make_user("Bagouze", "premier@example.com"))
        .await
        .unwrap();

    let result = repo
        .create(&make_user("Bagouze", "second@example.com"))
        .await;

    assert!(matches!(
        result,
        Err(RepositoryError::CoachNameAlreadyTaken)
    ));
}

#[sqlx::test]
async fn create_renvoie_email_already_taken(pool: PgPool) {
    let repo = UserRepository::new(pool);
    repo.create(&make_user("PremierCoach", "partage@example.com"))
        .await
        .unwrap();

    let result = repo
        .create(&make_user("SecondCoach", "partage@example.com"))
        .await;

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

/// Un coach qui ne se souvient plus de la casse de son nom doit pouvoir se
/// connecter et demander la réinitialisation de son mot de passe : les deux
/// parcours passent par cette recherche.
#[sqlx::test]
async fn find_by_coach_name_est_insensible_a_la_casse(pool: PgPool) {
    let repo = UserRepository::new(pool);
    repo.create(&make_user("Bagouze", "bagouze@example.com"))
        .await
        .unwrap();

    for saisie in ["bagouze", "BAGOUZE", "bAgOuZe"] {
        let found = repo.find_by_coach_name(saisie).await.unwrap();

        let found = found.unwrap_or_else(|| panic!("'{saisie}' aurait dû retrouver le compte"));
        assert_eq!(found.coach_name.into_inner(), "Bagouze");
    }
}

/// Corollaire de la recherche insensible à la casse : deux comptes ne
/// différant que par la casse rendraient le résultat ambigu. L'index unique
/// fonctionnel les interdit, et l'erreur reste bien attribuée au nom de coach.
#[sqlx::test]
async fn create_refuse_un_nom_deja_pris_dans_une_autre_casse(pool: PgPool) {
    let repo = UserRepository::new(pool);
    repo.create(&make_user("Bagouze", "premier@example.com"))
        .await
        .unwrap();

    let result = repo
        .create(&make_user("bagouze", "second@example.com"))
        .await;

    assert!(matches!(
        result,
        Err(RepositoryError::CoachNameAlreadyTaken)
    ));
}

/// La mise à jour vise le compte, pas la casse exacte du nom transmis.
#[sqlx::test]
async fn update_password_hash_est_insensible_a_la_casse(pool: PgPool) {
    let repo = UserRepository::new(pool);
    repo.create(&make_user("Bagouze", "bagouze@example.com"))
        .await
        .unwrap();

    repo.update_password_hash("BAGOUZE", "nouveau_hash")
        .await
        .unwrap();

    let found = repo.find_by_coach_name("Bagouze").await.unwrap().unwrap();
    assert_eq!(found.password_hash, "nouveau_hash");
}
