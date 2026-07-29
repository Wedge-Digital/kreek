use crate::app::teams::domain::team::GamePhase;
use crate::app::teams::ports::{
    basket_phase_key, IPhaseBasketRepository, PhaseBasketState, RepositoryError,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

pub struct PhaseBasketRepository {
    pool: PgPool,
}

impl PhaseBasketRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// La table ne porte qu'une ligne par équipe et par phase : la concurrence ne
/// peut donc pas être détectée par une contrainte d'unicité comme dans l'event
/// store. Elle l'est par le `WHERE version = $` pour une mise à jour, et par le
/// conflit de clé primaire pour une création — deux mécanismes, une seule erreur.
fn est_conflit_de_cle(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.constraint() == Some("teams__phase_baskets_pkey"))
}

#[async_trait]
impl IPhaseBasketRepository for PhaseBasketRepository {
    async fn load(
        &self,
        team_id: &str,
        phase: &GamePhase,
    ) -> Result<Option<PhaseBasketState>, RepositoryError> {
        let cle = basket_phase_key(phase)?;

        let row = sqlx::query(
            "SELECT space_id, state, version FROM teams__phase_baskets
             WHERE team_id = $1 AND phase = $2",
        )
        .bind(team_id)
        .bind(cle)
        .fetch_optional(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(row.map(|r| PhaseBasketState {
            team_id: team_id.to_string(),
            space_id: r.get("space_id"),
            phase: phase.clone(),
            state: r.get("state"),
            version: r.get::<i32, _>("version") as u32,
        }))
    }

    async fn save(
        &self,
        basket: &PhaseBasketState,
        expected_version: u32,
    ) -> Result<u32, RepositoryError> {
        let cle = basket_phase_key(&basket.phase)?;

        // Version zéro : le panier n'existe pas encore. Deux onglets qui
        // ajoutent une première ligne en même temps se disputent la clé
        // primaire, et le perdant doit voir un conflit de concurrence — pas une
        // erreur base incompréhensible.
        if expected_version == 0 {
            sqlx::query(
                "INSERT INTO teams__phase_baskets (team_id, phase, space_id, state, version)
                 VALUES ($1, $2, $3, $4, 1)",
            )
            .bind(&basket.team_id)
            .bind(cle)
            .bind(&basket.space_id)
            .bind(&basket.state)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if est_conflit_de_cle(&e) {
                    RepositoryError::ConcurrentWrite
                } else {
                    RepositoryError::Database(e)
                }
            })?;
            return Ok(1);
        }

        let resultat = sqlx::query(
            "UPDATE teams__phase_baskets
                SET state = $3, version = version + 1, updated_at = now()
              WHERE team_id = $1 AND phase = $2 AND version = $4",
        )
        .bind(&basket.team_id)
        .bind(cle)
        .bind(&basket.state)
        .bind(expected_version as i32)
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        // Zéro ligne touchée : soit la version a bougé sous nos pieds, soit le
        // panier a été purgé entre-temps. Dans les deux cas, l'appelant
        // travaillait sur un état périmé.
        if resultat.rows_affected() == 0 {
            return Err(RepositoryError::ConcurrentWrite);
        }
        Ok(expected_version + 1)
    }

    async fn delete(&self, team_id: &str, phase: &GamePhase) -> Result<(), RepositoryError> {
        let cle = basket_phase_key(phase)?;
        sqlx::query("DELETE FROM teams__phase_baskets WHERE team_id = $1 AND phase = $2")
            .bind(team_id)
            .bind(cle)
            .execute(&self.pool)
            .await
            .map_err(RepositoryError::Database)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .ok()
    }

    fn panier(team_id: &str, lignes: serde_json::Value) -> PhaseBasketState {
        PhaseBasketState {
            team_id: team_id.to_string(),
            space_id: "space-1".to_string(),
            phase: GamePhase::Recruitment,
            state: lignes,
            version: 0,
        }
    }

    #[tokio::test]
    async fn un_panier_absent_se_charge_en_none() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = PhaseBasketRepository::new(pool);
        let inconnu = ulid::Ulid::new().to_string();

        let charge = repo.load(&inconnu, &GamePhase::Recruitment).await.unwrap();
        assert!(
            charge.is_none(),
            "aucune ligne accumulée, donc aucun panier"
        );
    }

    #[tokio::test]
    async fn le_panier_se_cree_puis_se_recharge_tel_quel() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = PhaseBasketRepository::new(pool);
        let team_id = ulid::Ulid::new().to_string();
        let lignes = serde_json::json!([{"line_id": "PIETAILLE", "qty": 2}]);

        let v = repo
            .save(&panier(&team_id, lignes.clone()), 0)
            .await
            .unwrap();
        assert_eq!(v, 1, "la création part à la version 1");

        let charge = repo
            .load(&team_id, &GamePhase::Recruitment)
            .await
            .unwrap()
            .expect("le panier existe");
        assert_eq!(charge.state, lignes);
        assert_eq!(charge.version, 1);
        assert_eq!(charge.space_id, "space-1");
    }

    /// La garde de la carte : deux onglets qui sauvent depuis la même version
    /// lue, seul le premier passe.
    #[tokio::test]
    async fn deux_save_sur_la_meme_version_attendue_le_second_echoue() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = PhaseBasketRepository::new(pool);
        let team_id = ulid::Ulid::new().to_string();

        repo.save(&panier(&team_id, serde_json::json!([])), 0)
            .await
            .unwrap();

        let onglet_a = repo
            .save(&panier(&team_id, serde_json::json!(["a"])), 1)
            .await;
        let onglet_b = repo
            .save(&panier(&team_id, serde_json::json!(["b"])), 1)
            .await;

        assert_eq!(onglet_a.unwrap(), 2);
        assert!(
            matches!(onglet_b, Err(RepositoryError::ConcurrentWrite)),
            "le second travaillait sur un état périmé"
        );
    }

    /// La course à la **création** doit donner la même erreur que celle à la
    /// mise à jour — pas une erreur base incompréhensible.
    #[tokio::test]
    async fn deux_creations_concurrentes_donnent_concurrent_write() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = PhaseBasketRepository::new(pool);
        let team_id = ulid::Ulid::new().to_string();

        repo.save(&panier(&team_id, serde_json::json!([])), 0)
            .await
            .unwrap();
        let second = repo.save(&panier(&team_id, serde_json::json!([])), 0).await;

        assert!(matches!(second, Err(RepositoryError::ConcurrentWrite)));
    }

    #[tokio::test]
    async fn les_deux_phases_coexistent_sans_se_marcher_dessus() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = PhaseBasketRepository::new(pool);
        let team_id = ulid::Ulid::new().to_string();

        repo.save(&panier(&team_id, serde_json::json!(["recrutement"])), 0)
            .await
            .unwrap();
        let mut renvois = panier(&team_id, serde_json::json!(["renvois"]));
        renvois.phase = GamePhase::Dismissals;
        repo.save(&renvois, 0).await.unwrap();

        repo.delete(&team_id, &GamePhase::Recruitment)
            .await
            .unwrap();

        assert!(repo
            .load(&team_id, &GamePhase::Recruitment)
            .await
            .unwrap()
            .is_none());
        assert!(repo
            .load(&team_id, &GamePhase::Dismissals)
            .await
            .unwrap()
            .is_some());
    }

    /// Une phase sans panier possible est un bug d'appelant : elle doit se
    /// voir, pas être avalée en silence.
    #[tokio::test]
    async fn une_phase_sans_panier_est_rejetee() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = PhaseBasketRepository::new(pool);

        let r = repo.load("t1", &GamePhase::OffSeason).await;
        assert!(matches!(r, Err(RepositoryError::PhaseWithoutBasket(_))));
    }
}
