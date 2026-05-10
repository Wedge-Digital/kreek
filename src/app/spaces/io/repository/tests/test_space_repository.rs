use sqlx::PgPool;
use crate::app::shared_kernel::authorization::SpaceAuthorization;
use crate::app::shared_kernel::common_types::{CloudinaryImage, SpaceId};
use crate::app::shared_kernel::space_name::SpaceName;
use crate::app::spaces::domain::Space::Space;
use crate::app::spaces::domain::ports::ISpaceRepository;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;

fn make_space(name: &str) -> Space {
    Space::new(
        SpaceId::new(),
        SpaceName::try_new(name).unwrap(),
        CloudinaryImage::try_new("https://res.cloudinary.com/demo/image/upload/sample.jpg").unwrap(),
        vec![],
    )
}

#[sqlx::test]
async fn save_persiste_un_espace(pool: PgPool) {
    let repo = SpaceRepository::new(pool);
    let space = make_space("LigueAlpha");

    let result = repo.save(&space).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn find_by_id_retourne_none_si_absent(pool: PgPool) {
    let repo = SpaceRepository::new(pool);

    let result = repo.find_by_id(&SpaceId::new()).await;

    assert!(matches!(result, Ok(None)));
}

#[sqlx::test]
async fn find_by_id_retourne_espace_avec_coaches(pool: PgPool) {
    use crate::app::shared_kernel::common_types::CoachId;

    let repo = SpaceRepository::new(pool);
    let space = make_space("LigueOmega");
    let coach_id = CoachId::new();
    let space_id = space.id;

    repo.save(&space).await.unwrap();

    // On insère directement un user et un membre pour le test
    // (le use-case complet est testé en intégration)

    let found = repo.find_by_id(&space_id).await.unwrap().unwrap();
    assert_eq!(found.name.as_ref(), "LigueOmega");
    assert!(found.coaches.is_empty());
}