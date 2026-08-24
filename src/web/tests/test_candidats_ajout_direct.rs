//! La liste des candidats à l'ajout direct.
//!
//! Deux tests portent tout le reste : le seuil s'applique **avant** la lecture,
//! et les trois états sont bien trois.

use crate::cli::seed_e2e::SIMPLE_COACH_NAME;
use crate::web::test_harness::Harnais;
use axum::http::StatusCode;

/// Identifiants de test valides au sens ULID.
///
/// L'alphabet de Crockford exclut `I`, `L`, `O` et `U` — un identifiant
/// « parlant » comme `01JSOLITAIRE…` est donc refusé par le value object, et le
/// contrôleur rend 400. Un test d'autorisation passerait quand même, la garde
/// répondant avant la validation : il passerait **pour la mauvaise raison**.

async fn contexte(pool: &sqlx::PgPool) -> String {
    crate::cli::seed_e2e::execute(pool).await.expect("seed e2e");
    sqlx::query_scalar(
        "SELECT m.space_id FROM spaces__user_space m
         JOIN auth__users u ON u.id = m.coach_id
         WHERE u.coach_name = 'DevCoach' AND m.profile = 'SpaceAdmin' LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("l'espace administré par DevCoach")
}

/// Le seuil s'applique avant la lecture.
///
/// Un `q` d'un caractère ne doit **rien** chercher : un seuil qui filtrerait le
/// résultat aurait déjà interrogé l'annuaire, et le garde-fou serait décoratif.
/// L'observable est l'état rendu, qui ne dit rien d'une recherche.
#[sqlx::test]
async fn un_seul_caractere_rend_l_etat_sous_seuil(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!("/app/{space}/admin/widgets/candidates?q=D"))
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains("au moins deux caractères"), "{}", r.corps);
    assert!(
        !r.corps.contains("Créez-lui un compte"),
        "l'état sous-seuil ne propose pas de créer un compte : {}",
        r.corps
    );
    assert!(!r.corps.contains("sac-row"), "aucun candidat n'est rendu");
}

/// L'état vide, lui, propose la création — et c'est ce qui le distingue.
#[sqlx::test]
async fn une_recherche_sans_resultat_propose_de_creer_un_compte(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!(
            "/app/{space}/admin/widgets/candidates?q=ZzzPersonneIci"
        ))
        .await;

    assert!(r.corps.contains("Créez-lui un compte"), "{}", r.corps);
    assert!(
        r.corps.contains("ZzzPersonneIci"),
        "la requête est rappelée"
    );
    assert!(!r.corps.contains("au moins deux caractères"));
}

#[sqlx::test]
async fn un_membre_de_l_espace_est_rendu_avec_son_badge(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!(
            "/app/{space}/admin/widgets/candidates?q={}",
            SIMPLE_COACH_NAME.replace(' ', "+")
        ))
        .await;

    assert!(r.corps.contains("Déjà membre"), "{}", r.corps);
    assert!(
        !r.corps.contains("sac-btn"),
        "une ligne « déjà membre » ne porte ni bouton ni sélecteur : {}",
        r.corps
    );
}

#[sqlx::test]
async fn un_non_membre_est_rendu_avec_son_selecteur_et_son_bouton(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    // Un coach de la plateforme qui n'est membre d'aucun espace.
    sqlx::query(
        "INSERT INTO spaces__user_cache (id, coach_name, coach_icon, email)
         VALUES ('01JEEEEEEEEEEEEEEEEEEEEEEE', 'Solitaire', NULL, 'solitaire@bb.club')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!(
            "/app/{space}/admin/widgets/candidates?q=Solitaire"
        ))
        .await;

    assert!(r.corps.contains("sac-btn"), "{}", r.corps);
    assert!(r.corps.contains("kreek-select"));
    assert!(!r.corps.contains("Déjà membre"));
}

#[sqlx::test]
async fn un_membre_simple_n_atteint_pas_la_liste_des_candidats(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, SIMPLE_COACH_NAME).await;

    let r = app
        .get(&format!("/app/{space}/admin/widgets/candidates?q=Dev"))
        .await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
}

// ── L'ajout d'un coach déjà inscrit (carte 382) ─────────────────────────────

async fn coach_libre(pool: &sqlx::PgPool, id: &str, nom: &str) {
    sqlx::query(
        "INSERT INTO spaces__user_cache (id, coach_name, coach_icon, email)
         VALUES ($1, $2, NULL, $3)",
    )
    .bind(id)
    .bind(nom)
    .bind(format!("{nom}@bb.club"))
    .execute(pool)
    .await
    .unwrap();
}

/// La ligne est **re-rendue avec son badge**, pas retirée.
///
/// Le coach existe toujours dans l'annuaire ; le faire disparaître laisserait
/// croire à une suppression.
#[sqlx::test]
async fn ajouter_un_coach_rerend_sa_ligne_avec_le_badge(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    coach_libre(&pool, "01JBBBBBBBBBBBBBBBBBBBBBBB", "Ajoute").await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(
            &format!("/app/{space}/admin/members/add"),
            "coach_id=01JBBBBBBBBBBBBBBBBBBBBBBB&profile=SpaceUser",
        )
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(r.corps.contains("Déjà membre"), "{}", r.corps);
    assert!(
        !r.corps.contains("sac-btn"),
        "la ligne ne propose plus d'ajouter"
    );
}

/// Le contrat du journal de session.
///
/// Le `name` voyage dans l'événement pour une seule raison : le journal affiche
/// depuis ce payload, sans relire. Sans lui, il retomberait dans la course du
/// cache d'utilisateurs qu'il est là pour masquer.
#[sqlx::test]
async fn l_ajout_declenche_member_added_avec_le_pseudo(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    coach_libre(&pool, "01JCCCCCCCCCCCCCCCCCCCCCCC", "Payload").await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(
            &format!("/app/{space}/admin/members/add"),
            "coach_id=01JCCCCCCCCCCCCCCCCCCCCCCC&profile=SpaceUser",
        )
        .await;

    let entete = r.entete("hx-trigger").expect("l'en-tête doit être posé");
    assert!(entete.contains("memberAdded"), "{entete}");
    assert!(entete.contains("01JCCCCCCCCCCCCCCCCCCCCCCC"), "{entete}");
    assert!(
        entete.contains(r#""name":"Payload""#),
        "le pseudo doit voyager : {entete}"
    );
}

#[sqlx::test]
async fn ajouter_un_coach_deja_membre_rend_409(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let deja: String = sqlx::query_scalar("SELECT id FROM auth__users WHERE coach_name = $1")
        .bind(SIMPLE_COACH_NAME)
        .fetch_one(&pool)
        .await
        .unwrap();
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .post_htmx(
            &format!("/app/{space}/admin/members/add"),
            &format!("coach_id={deja}&profile=SpaceUser"),
        )
        .await;

    assert_eq!(r.statut, StatusCode::CONFLICT);
    assert!(r.corps.contains("déjà membre"), "{}", r.corps);
}

/// Le grisage de la liste des candidats est une politesse, pas une garde.
#[sqlx::test]
async fn un_membre_simple_ne_peut_pas_ajouter(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    coach_libre(&pool, "01JDDDDDDDDDDDDDDDDDDDDDDD", "Interdit").await;
    let app = Harnais::connecte_en_tant_que(pool, SIMPLE_COACH_NAME).await;

    let r = app
        .post_htmx(
            &format!("/app/{space}/admin/members/add"),
            "coach_id=01JDDDDDDDDDDDDDDDDDDDDDDD&profile=SpaceAdmin",
        )
        .await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
}

// ── Le panneau de création de compte (carte 383) ────────────────────────────

/// Le fragment de l'hôte est bien injecté dans le panneau.
///
/// Ce BC ne sait pas ce qu'il contient — il vérifie seulement qu'il est là, et
/// qu'il y a joint ce qui lui appartient : le sélecteur de profil.
#[sqlx::test]
async fn le_panneau_injecte_le_formulaire_de_l_hote_et_son_selecteur(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!("/app/{space}/admin/widgets/create-coach"))
        .await;

    assert_eq!(r.statut, StatusCode::OK);
    assert!(
        r.corps.contains(r#"name="coach_name""#),
        "le formulaire de l'hôte doit être injecté : {}",
        r.corps
    );
    assert!(
        r.corps.contains("space-admin-creation-profil"),
        "et le sélecteur de profil, qui appartient à ce BC : {}",
        r.corps
    );
}

/// La répartition du terme cherché est décidée **ici**.
#[sqlx::test]
async fn le_terme_cherche_est_reparti_selon_la_presence_d_un_arobase(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let pseudo = app
        .get(&format!(
            "/app/{space}/admin/widgets/create-coach?q=NurgleFan"
        ))
        .await;
    assert!(pseudo.corps.contains(r#"value="NurgleFan""#));

    let adresse = app
        .get(&format!(
            "/app/{space}/admin/widgets/create-coach?q=nurgle%40bb.club"
        ))
        .await;
    assert!(
        adresse.corps.contains(r#"value="nurgle@bb.club""#),
        "une saisie contenant un @ part dans le champ e-mail : {}",
        adresse.corps
    );
}

/// L'écoute du contrat, côté récepteur.
///
/// Le harnais ne peut pas vérifier que les deux bords s'accordent — seul un test
/// de bout en bout le peut. Il vérifie que ce bord-ci écoute bien le nom convenu
/// et lit les deux clés attendues.
#[sqlx::test]
async fn le_panneau_ecoute_l_evenement_et_poste_l_appartenance(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, "DevCoach").await;

    let r = app
        .get(&format!("/app/{space}/admin/widgets/create-coach"))
        .await;

    assert!(r.corps.contains("account-created.window"), "{}", r.corps);
    assert!(r.corps.contains("$event.detail.coach_id"), "{}", r.corps);
    assert!(
        r.corps.contains(&format!("/app/{space}/admin/members/add")),
        "et poste vers l'ajout : {}",
        r.corps
    );
}

#[sqlx::test]
async fn un_membre_simple_n_atteint_pas_le_panneau_de_creation(pool: sqlx::PgPool) {
    let space = contexte(&pool).await;
    let app = Harnais::connecte_en_tant_que(pool, SIMPLE_COACH_NAME).await;

    let r = app
        .get(&format!("/app/{space}/admin/widgets/create-coach"))
        .await;

    assert_eq!(r.statut, StatusCode::FORBIDDEN);
}
