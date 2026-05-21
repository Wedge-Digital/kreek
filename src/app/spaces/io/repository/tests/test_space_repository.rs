use sqlx::PgPool;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::space_name::SpaceName;
use crate::app::spaces::domain::space::Space;
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
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
    let repo = SpaceRepository::new(pool);
    let space = make_space("LigueOmega");
    let space_id = space.id;

    repo.save(&space).await.unwrap();

    let found = repo.find_by_id(&space_id).await.unwrap().unwrap();
    assert_eq!(found.name.as_ref(), "LigueOmega");
    assert!(found.coaches.is_empty());
}

#[sqlx::test]
async fn join_spaces_insere_plusieurs_membres_en_une_requete(pool: PgPool) {
    let repo = SpaceRepository::new(pool);
    let space1 = make_space("AlphaTeam");
    let space2 = make_space("BetaTeam");
    let coach_id = CoachId::new();
    let id1 = space1.id;
    let id2 = space2.id;

    repo.save(&space1).await.unwrap();
    repo.save(&space2).await.unwrap();

    let result = repo.join_spaces(&[id1, id2], &coach_id).await;
    assert!(result.is_ok());

    let profile1 = repo.find_member_profile(&coach_id, &id1).await.unwrap();
    let profile2 = repo.find_member_profile(&coach_id, &id2).await.unwrap();
    assert!(matches!(profile1, Some(SpaceProfile::SpaceUser)));
    assert!(matches!(profile2, Some(SpaceProfile::SpaceUser)));
}

#[sqlx::test]
async fn join_spaces_ignore_les_doublons(pool: PgPool) {
    let repo = SpaceRepository::new(pool);
    let space = make_space("GammaTeam");
    let coach_id = CoachId::new();
    let id = space.id;

    repo.save(&space).await.unwrap();
    repo.join_spaces(&[id], &coach_id).await.unwrap();

    let result = repo.join_spaces(&[id], &coach_id).await;
    assert!(result.is_ok());
}