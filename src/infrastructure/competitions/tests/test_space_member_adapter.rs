//! R7, vérifiée là où elle se joue : dans la requête.
//!
//! Les tests de `notification_recipients` montrent que le service n'invente
//! aucun destinataire hors de la liste qu'on lui donne. Ils ne peuvent rien dire
//! de la liste elle-même — une doublure rend ce qu'on lui a écrit. Or c'est
//! **l'adapter** qui borne par espace, et c'est là que R7 tomberait sans bruit :
//! une jointure fausse rendrait tous les coachs de la plateforme, et le service
//! les accepterait tous.

use crate::app::competitions::ports::ICompetitionSpaceMemberPort;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::shared_kernel::identity::sulid::SUlid;
use crate::app::spaces::io::repository::space_repository::SpaceRepository;
use crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository;
use crate::infrastructure::competitions::space_member_adapter::SpaceMemberAdapter;
use std::sync::Arc;

fn adaptateur(pool: sqlx::PgPool) -> SpaceMemberAdapter {
    SpaceMemberAdapter::new(
        Arc::new(SpaceRepository::new(pool.clone())),
        Arc::new(SpaceUserCacheRepository::new(pool)),
    )
}

/// Un espace voisin, avec son propre coach. Sans borne, il ressortirait.
async fn espace_voisin(pool: &sqlx::PgPool) -> (String, String) {
    let space_id = SUlid::new().to_string();
    let coach_id = SUlid::new().to_string();

    sqlx::query("INSERT INTO spaces (id, space_name, space_icon_path, created_at) VALUES ($1, 'Espace Voisin', '', now())")
        .bind(&space_id)
        .execute(pool)
        .await
        .expect("espace voisin");
    sqlx::query(
        "INSERT INTO spaces__user_cache (id, coach_name, email, created_at)
         VALUES ($1, 'CoachVoisin', 'voisin@example.test', now())",
    )
    .bind(&coach_id)
    .execute(pool)
    .await
    .expect("coach voisin");
    sqlx::query(
        "INSERT INTO spaces__user_space (space_id, coach_id, profile, created_at)
         VALUES ($1, $2, 'member', now())",
    )
    .bind(&space_id)
    .bind(&coach_id)
    .execute(pool)
    .await
    .expect("appartenance voisine");

    (space_id, coach_id)
}

#[sqlx::test]
async fn les_membres_rendus_sont_ceux_de_l_espace_demande(pool: sqlx::PgPool) {
    crate::cli::seed_e2e::execute(&pool)
        .await
        .expect("seed e2e");
    let (espace_e2e,): (String,) =
        sqlx::query_as("SELECT id FROM spaces WHERE space_name = 'Espace E2E'")
            .fetch_one(&pool)
            .await
            .expect("espace E2E seedé");
    let (_voisin, coach_voisin) = espace_voisin(&pool).await;

    let membres = adaptateur(pool)
        .list_space_members(&SpaceId::try_new(&espace_e2e).unwrap())
        .await;

    assert!(!membres.is_empty(), "l'espace E2E a des membres seedés");
    assert!(
        !membres.iter().any(|m| m.coach_id == coach_voisin),
        "un coach d'un autre espace ne doit jamais ressortir — c'est R7"
    );
    assert!(
        membres.iter().all(|m| !m.email.is_empty()),
        "sans adresse, le destinataire est inutilisable"
    );
}

#[sqlx::test]
async fn un_espace_sans_membre_rend_une_liste_vide_et_non_une_erreur(pool: sqlx::PgPool) {
    let space_id = SUlid::new().to_string();
    sqlx::query("INSERT INTO spaces (id, space_name, space_icon_path, created_at) VALUES ($1, 'Espace Vide', '', now())")
        .bind(&space_id)
        .execute(&pool)
        .await
        .expect("espace vide");

    let membres = adaptateur(pool)
        .list_space_members(&SpaceId::try_new(&space_id).unwrap())
        .await;

    // Le cron doit pouvoir enchaîner : une saison sans destinataire n'est pas
    // une panne.
    assert!(membres.is_empty());
}
