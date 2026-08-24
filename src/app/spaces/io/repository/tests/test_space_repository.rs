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

// ── Lire, modifier et retirer une appartenance (carte 366) ───────────────────
//
// `spaces__user_space` a pour clé primaire `(space_id, coach_id)`. Une écriture
// qui omettrait `space_id` toucherait le même coach dans **tous** ses espaces,
// sans erreur — et passerait tout test qui n'utilise qu'un seul espace. D'où les
// deux tests « dans un autre espace », qui sont la raison d'être des cinq.

#[sqlx::test]
async fn list_members_with_profile_rend_le_profil_de_chaque_membre(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let space = make_space("LigueProfils");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();

    let patron = semer_coach(&pool, "Patron", None).await;
    let simple = semer_coach(&pool, "Simple", None).await;
    repo.add_member(&space_id, &patron, &SpaceProfile::SpaceAdmin)
        .await
        .unwrap();
    repo.add_member(&space_id, &simple, &SpaceProfile::SpaceUser)
        .await
        .unwrap();

    let membres = repo.list_members_with_profile(&space_id).await.unwrap();

    assert_eq!(membres.len(), 2);
    let profils: Vec<&str> = membres.iter().map(|m| m.profile.as_str()).collect();
    assert!(profils.contains(&"SpaceAdmin"));
    assert!(profils.contains(&"SpaceUser"));
    assert!(membres.iter().all(|m| m.email.ends_with("@example.com")));
}

#[sqlx::test]
async fn list_members_with_profile_trie_par_pseudo(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let space = make_space("LigueTri");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();

    // Insérés dans le désordre : le tri ne doit rien devoir à l'ordre d'entrée.
    for nom in ["Zoltan", "Alpha", "Melchior"] {
        let id = semer_coach(&pool, nom, None).await;
        repo.add_member(&space_id, &id, &SpaceProfile::SpaceUser)
            .await
            .unwrap();
    }

    let noms: Vec<String> = repo
        .list_members_with_profile(&space_id)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.coach_name)
        .collect();

    assert_eq!(noms, vec!["Alpha", "Melchior", "Zoltan"]);
}

#[sqlx::test]
async fn list_members_with_profile_ne_rend_que_les_membres_de_l_espace_demande(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let ici = make_space("LigueIci");
    let ailleurs = make_space("LigueAilleurs");
    let (id_ici, id_ailleurs) = (*ici.id(), *ailleurs.id());
    repo.save(&ici).await.unwrap();
    repo.save(&ailleurs).await.unwrap();

    let coach = semer_coach(&pool, "Nomade", None).await;
    repo.add_member(&id_ici, &coach, &SpaceProfile::SpaceUser)
        .await
        .unwrap();
    let autre = semer_coach(&pool, "Sedentaire", None).await;
    repo.add_member(&id_ailleurs, &autre, &SpaceProfile::SpaceAdmin)
        .await
        .unwrap();

    let membres = repo.list_members_with_profile(&id_ici).await.unwrap();

    assert_eq!(membres.len(), 1);
    assert_eq!(membres[0].coach_name, "Nomade");
}

#[sqlx::test]
async fn update_member_profile_ne_touche_pas_le_meme_coach_dans_un_autre_espace(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let cible = make_space("LigueCible");
    let temoin = make_space("LigueTemoin");
    let (id_cible, id_temoin) = (*cible.id(), *temoin.id());
    repo.save(&cible).await.unwrap();
    repo.save(&temoin).await.unwrap();

    // Le **même** coach, membre des deux espaces avec le même profil.
    let coach = semer_coach(&pool, "Bilocalise", None).await;
    repo.add_member(&id_cible, &coach, &SpaceProfile::SpaceUser)
        .await
        .unwrap();
    repo.add_member(&id_temoin, &coach, &SpaceProfile::SpaceUser)
        .await
        .unwrap();

    repo.update_member_profile(&id_cible, &coach, &SpaceProfile::SpaceAdmin)
        .await
        .unwrap();

    let dans_cible = repo.list_members_with_profile(&id_cible).await.unwrap();
    let dans_temoin = repo.list_members_with_profile(&id_temoin).await.unwrap();

    assert_eq!(dans_cible[0].profile, "SpaceAdmin");
    assert_eq!(
        dans_temoin[0].profile, "SpaceUser",
        "l'écriture a débordé sur un autre espace — `space_id` manque au WHERE"
    );
}

#[sqlx::test]
async fn delete_member_ne_retire_pas_le_meme_coach_d_un_autre_espace(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let cible = make_space("LigueCibleD");
    let temoin = make_space("LigueTemoinD");
    let (id_cible, id_temoin) = (*cible.id(), *temoin.id());
    repo.save(&cible).await.unwrap();
    repo.save(&temoin).await.unwrap();

    let coach = semer_coach(&pool, "Bilocalise", None).await;
    repo.add_member(&id_cible, &coach, &SpaceProfile::SpaceUser)
        .await
        .unwrap();
    repo.add_member(&id_temoin, &coach, &SpaceProfile::SpaceUser)
        .await
        .unwrap();

    repo.delete_member(&id_cible, &coach).await.unwrap();

    assert!(repo
        .list_members_with_profile(&id_cible)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        repo.list_members_with_profile(&id_temoin)
            .await
            .unwrap()
            .len(),
        1,
        "le retrait a débordé sur un autre espace — `space_id` manque au WHERE"
    );
}

// ── Recherche dans l'annuaire de la plateforme (carte 377) ───────────────────
//
// Le piège est dans la jointure. `m.space_id = $1` vit dans la condition de
// jointure ; l'y retirer transforme la jointure externe en interne, et la
// recherche ne rend plus que les membres — l'exact inverse du besoin, sans
// erreur et avec une liste plausible. Le test « membre d'un autre espace » est
// le seul qui l'attrape.

const PLAFOND: i64 = 20;

#[sqlx::test]
async fn la_recherche_marque_les_membres_de_l_espace(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let space = make_space("LigueRecherche");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();

    let dedans = semer_coach(&pool, "Dedans", None).await;
    semer_coach(&pool, "Dehors", None).await;
    repo.add_member(&space_id, &dedans, &SpaceProfile::SpaceUser)
        .await
        .unwrap();

    let trouves = repo
        .search_platform_coaches(&space_id, "De", PLAFOND)
        .await
        .unwrap();

    let dedans_row = trouves.iter().find(|c| c.coach_name == "Dedans").unwrap();
    let dehors_row = trouves.iter().find(|c| c.coach_name == "Dehors").unwrap();
    assert!(dedans_row.est_membre, "un membre est rendu, et marqué");
    assert!(!dehors_row.est_membre);
}

/// Le test qui attrape le piège.
///
/// Un coach membre d'un **autre** espace doit apparaître comme non-membre de
/// celui-ci. Si `space_id` glissait dans le `WHERE`, il n'apparaîtrait pas du
/// tout — et les deux autres tests passeraient quand même.
#[sqlx::test]
async fn un_membre_d_un_autre_espace_est_rendu_comme_non_membre(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let ici = make_space("LigueIciR");
    let ailleurs = make_space("LigueAilleursR");
    let (id_ici, id_ailleurs) = (*ici.id(), *ailleurs.id());
    repo.save(&ici).await.unwrap();
    repo.save(&ailleurs).await.unwrap();

    let nomade = semer_coach(&pool, "Nomade", None).await;
    repo.add_member(&id_ailleurs, &nomade, &SpaceProfile::SpaceAdmin)
        .await
        .unwrap();

    let trouves = repo
        .search_platform_coaches(&id_ici, "Nomade", PLAFOND)
        .await
        .unwrap();

    assert_eq!(
        trouves.len(),
        1,
        "il doit être trouvé : une jointure interne l'aurait fait disparaître"
    );
    assert!(
        !trouves[0].est_membre,
        "et rendu comme non-membre de l'espace courant"
    );
}

#[sqlx::test]
async fn la_recherche_porte_aussi_sur_l_email(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let space = make_space("LigueEmail");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();
    semer_coach(&pool, "Anonyme", None).await;

    let trouves = repo
        .search_platform_coaches(&space_id, "Anonyme@example", PLAFOND)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1, "l'email est cherché comme le pseudo");
}

#[sqlx::test]
async fn le_plafond_borne_le_nombre_de_resultats(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let space = make_space("LiguePlafond");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();
    for i in 0..25 {
        semer_coach(&pool, &format!("Foule{i:02}"), None).await;
    }

    let trouves = repo
        .search_platform_coaches(&space_id, "Foule", PLAFOND)
        .await
        .unwrap();

    assert_eq!(trouves.len(), PLAFOND as usize);
}

#[sqlx::test]
async fn les_resultats_sont_tries_par_pseudo(pool: PgPool) {
    let repo = SpaceRepository::new(pool.clone());
    let space = make_space("LigueTriR");
    let space_id = *space.id();
    repo.save(&space).await.unwrap();
    for nom in ["TriZoltan", "TriAlpha", "TriMelchior"] {
        semer_coach(&pool, nom, None).await;
    }

    let noms: Vec<String> = repo
        .search_platform_coaches(&space_id, "Tri", PLAFOND)
        .await
        .unwrap()
        .into_iter()
        .map(|c| c.coach_name)
        .collect();

    assert_eq!(noms, vec!["TriAlpha", "TriMelchior", "TriZoltan"]);
}
