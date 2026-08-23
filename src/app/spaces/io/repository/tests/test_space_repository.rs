use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::spaces::domain::space::Space;
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;
use sqlx::PgPool;

fn make_space(name: &str) -> Space {
    Space::new(
        SpaceId::new(),
        SpaceName::try_new(name).unwrap(),
        CloudinaryImage::try_new("https://res.cloudinary.com/demo/image/upload/sample.jpg")
            .unwrap(),
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
    let space_id = *space.id();

    repo.save(&space).await.unwrap();

    let found = repo.find_by_id(&space_id).await.unwrap().unwrap();
    assert_eq!(found.name().as_ref(), "LigueOmega");
    assert!(found.coaches().is_empty());
}

#[sqlx::test]
async fn join_spaces_insere_plusieurs_membres_en_une_requete(pool: PgPool) {
    let repo = SpaceRepository::new(pool);
    let space1 = make_space("AlphaTeam");
    let space2 = make_space("BetaTeam");
    let coach_id = CoachId::new();
    let id1 = *space1.id();
    let id2 = *space2.id();

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
    let id = *space.id();

    repo.save(&space).await.unwrap();
    repo.join_spaces(&[id], &coach_id).await.unwrap();

    let result = repo.join_spaces(&[id], &coach_id).await;
    assert!(result.is_ok());
}

// ── Le chargement des membres (carte 375) ────────────────────────────────────
//
// `find_by_id` sautait tout coach dont l'icône était nulle, en confondant
// « ligne sans membre » et « membre sans avatar ». Sur la base de démonstration,
// où aucun des trente-huit membres n'a d'avatar, l'agrégat se chargeait
// systématiquement vide de ses coachs.

/// Sème un coach dans le cache du BC, avec ou sans avatar.
///
/// L'insertion est écrite ici plutôt que déléguée à `add_user`, et ce n'est pas
/// un raccourci : `insert_user.sql` écrit `coach_icon` en `NULL` **en dur** et
/// ignore le champ qu'on lui passe. Passer par le dépôt ne permettrait donc pas
/// de construire le cas « membre avec avatar », qui est la moitié de ce qu'on
/// veut vérifier ici.
///
/// Ce test porte sur `find_by_id`, pas sur l'alimentation du cache : il a le
/// droit de poser l'état qu'il veut lire.
async fn semer_coach(pool: &PgPool, nom: &str, avatar: Option<&str>) -> CoachId {
    let id = CoachId::new();
    sqlx::query(
        "INSERT INTO spaces__user_cache (id, coach_name, coach_icon, email) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id.to_string())
    .bind(nom)
    .bind(avatar)
    .bind(format!("{nom}@example.com"))
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test]
async fn un_espace_sans_membre_rend_un_agregat_a_coaches_vide(pool: PgPool) {
    let repo = SpaceRepository::new(pool);
    let space = make_space("LigueVide");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();

    let found = repo.find_by_id(&space_id).await.unwrap().unwrap();

    // `None` dirait « cet espace n'existe pas », ce qui est faux.
    assert!(found.coaches().is_empty());
}

#[sqlx::test]
async fn un_membre_sans_avatar_est_present_dans_l_agregat(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let space = make_space("LigueSansAvatar");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();

    let coach = semer_coach(&pool, "CoachNu", None).await;
    repo.add_member(&space_id, &coach, &SpaceProfile::SpaceAdmin)
        .await
        .unwrap();

    let found = repo.find_by_id(&space_id).await.unwrap().unwrap();

    // Le test qui aurait attrapé le défaut : il rendait zéro coach.
    assert_eq!(found.coaches().len(), 1, "un membre sans avatar a disparu");
    assert_eq!(found.coaches()[0].id, coach);
    assert!(found.coaches()[0].icon.is_none());
    assert_eq!(found.coaches()[0].profile, SpaceProfile::SpaceAdmin);
}

#[sqlx::test]
async fn trois_membres_dont_un_sans_avatar_en_rendent_trois(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let space = make_space("LigueMixte");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();

    let avatar = "https://res.cloudinary.com/demo/image/upload/coach.jpg";
    for (nom, icone) in [
        ("CoachUn", Some(avatar)),
        ("CoachDeux", None),
        ("CoachTrois", Some(avatar)),
    ] {
        let id = semer_coach(&pool, nom, icone).await;
        repo.add_member(&space_id, &id, &SpaceProfile::SpaceUser)
            .await
            .unwrap();
    }

    let found = repo.find_by_id(&space_id).await.unwrap().unwrap();

    assert_eq!(found.coaches().len(), 3);
    assert_eq!(
        found.coaches().iter().filter(|c| c.icon.is_none()).count(),
        1
    );
}
