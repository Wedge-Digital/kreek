#![cfg(test)]

use crate::app::players::io::repository::customisation_basket_repository::PgCustomisationBasketRepository;
use crate::app::players::ports::{
    CustomisationBasketState, ICustomisationBasketRepository, RepositoryError,
};
use sqlx::PgPool;

fn panier(player_id: &str, lignes: serde_json::Value) -> CustomisationBasketState {
    CustomisationBasketState {
        player_id: player_id.to_string(),
        space_id: "s1".to_string(),
        state: lignes,
        version: 0,
        // Ignoré à l'écriture : c'est la base qui pose `updated_at`.
        updated_at: time::OffsetDateTime::UNIX_EPOCH,
    }
}

#[sqlx::test]
async fn un_panier_absent_se_lit_comme_none(pool: PgPool) {
    let repo = PgCustomisationBasketRepository::new(pool);
    assert!(repo.load("inconnu").await.unwrap().is_none());
}

#[sqlx::test]
async fn save_puis_load_rend_les_memes_lignes(pool: PgPool) {
    let repo = PgCustomisationBasketRepository::new(pool);
    let lignes = serde_json::json!([{"Skill": {"id": "l1", "skill_id": "BLOCK"}}]);

    let version = repo.save(&panier("p1", lignes.clone()), 0).await.unwrap();
    assert_eq!(version, 1);

    let relu = repo.load("p1").await.unwrap().unwrap();
    assert_eq!(relu.state, lignes);
    assert_eq!(relu.version, 1);
    assert_eq!(relu.space_id, "s1");
}

/// La garde de version : deux onglets partis du même état, le second perd.
#[sqlx::test]
async fn une_version_perimee_est_refusee_en_ecriture_concurrente(pool: PgPool) {
    let repo = PgCustomisationBasketRepository::new(pool);
    repo.save(&panier("p1", serde_json::json!([])), 0)
        .await
        .unwrap();

    // Le premier onglet écrit et fait passer la version à 2.
    repo.save(&panier("p1", serde_json::json!(["a"])), 1)
        .await
        .unwrap();

    // Le second, parti de la version 1, arrive trop tard.
    let erreur = repo
        .save(&panier("p1", serde_json::json!(["b"])), 1)
        .await
        .unwrap_err();
    assert!(matches!(erreur, RepositoryError::ConcurrentWrite));

    // Et l'état du premier n'a pas été écrasé.
    assert_eq!(
        repo.load("p1").await.unwrap().unwrap().state,
        serde_json::json!(["a"])
    );
}

/// Deux créations concurrentes se disputent la clé primaire : le perdant doit
/// voir un conflit de concurrence, pas une erreur base incompréhensible.
#[sqlx::test]
async fn deux_creations_concurrentes_donnent_un_conflit_lisible(pool: PgPool) {
    let repo = PgCustomisationBasketRepository::new(pool);
    repo.save(&panier("p1", serde_json::json!([])), 0)
        .await
        .unwrap();

    let erreur = repo
        .save(&panier("p1", serde_json::json!([])), 0)
        .await
        .unwrap_err();
    assert!(matches!(erreur, RepositoryError::ConcurrentWrite));
}

/// L'annulation se clique deux fois sans produire de message d'échec.
#[sqlx::test]
async fn delete_est_idempotent(pool: PgPool) {
    let repo = PgCustomisationBasketRepository::new(pool);
    repo.save(&panier("p1", serde_json::json!([])), 0)
        .await
        .unwrap();

    repo.delete("p1").await.unwrap();
    assert!(repo.load("p1").await.unwrap().is_none());
    repo.delete("p1").await.unwrap();
    repo.delete("jamais-existe").await.unwrap();
}

/// `updated_at` doit **avancer** à chaque écriture : c'est lui qui porte la
/// péremption, et la fenêtre doit glisser sur l'activité réelle.
#[sqlx::test]
async fn updated_at_avance_a_chaque_ecriture(pool: PgPool) {
    let repo = PgCustomisationBasketRepository::new(pool.clone());
    repo.save(&panier("p1", serde_json::json!([])), 0)
        .await
        .unwrap();

    // On recule l'horodatage pour simuler un panier vieux d'une journée.
    sqlx::query(
        "UPDATE players__customisation_baskets
         SET updated_at = now() - interval '20 hours' WHERE player_id = $1",
    )
    .bind("p1")
    .execute(&pool)
    .await
    .unwrap();
    let ancien = repo.load("p1").await.unwrap().unwrap().updated_at;

    repo.save(&panier("p1", serde_json::json!(["a"])), 1)
        .await
        .unwrap();
    let nouveau = repo.load("p1").await.unwrap().unwrap().updated_at;

    assert!(
        nouveau > ancien,
        "une écriture doit repousser la péremption"
    );
}

/// Le lien entre la persistance et la règle domaine : un panier dont
/// `updated_at` dépasse la fenêtre est jugé périmé.
#[sqlx::test]
async fn un_panier_trop_vieux_est_juge_perime(pool: PgPool) {
    use crate::app::players::domain::customisation_basket::is_expired;

    let repo = PgCustomisationBasketRepository::new(pool.clone());
    repo.save(&panier("p1", serde_json::json!([])), 0)
        .await
        .unwrap();

    let frais = repo.load("p1").await.unwrap().unwrap();
    let maintenant = time::OffsetDateTime::now_utc();
    assert!(!is_expired(frais.updated_at, maintenant));

    sqlx::query(
        "UPDATE players__customisation_baskets
         SET updated_at = now() - interval '25 hours' WHERE player_id = $1",
    )
    .bind("p1")
    .execute(&pool)
    .await
    .unwrap();

    let vieux = repo.load("p1").await.unwrap().unwrap();
    assert!(is_expired(vieux.updated_at, maintenant));
}
