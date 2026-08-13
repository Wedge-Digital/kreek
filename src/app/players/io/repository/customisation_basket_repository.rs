use crate::app::players::ports::{
    CustomisationBasketState, ICustomisationBasketRepository, RepositoryError,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

pub struct PgCustomisationBasketRepository {
    pool: PgPool,
}

impl PgCustomisationBasketRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// La clé primaire est la seule contrainte de la table : un conflit dessus est
/// forcément deux créations concurrentes, jamais autre chose.
fn est_conflit_de_cle(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.constraint() == Some("players__customisation_baskets_pkey"))
}

#[async_trait]
impl ICustomisationBasketRepository for PgCustomisationBasketRepository {
    async fn load(
        &self,
        player_id: &str,
    ) -> Result<Option<CustomisationBasketState>, RepositoryError> {
        let row = sqlx::query(
            "SELECT player_id, space_id, state, version, updated_at
             FROM players__customisation_baskets WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(row.map(|r| CustomisationBasketState {
            player_id: r.get("player_id"),
            space_id: r.get("space_id"),
            state: r.get("state"),
            version: r.get::<i32, _>("version") as u32,
            updated_at: r.get("updated_at"),
        }))
    }

    async fn save(
        &self,
        basket: &CustomisationBasketState,
        expected_version: u32,
    ) -> Result<u32, RepositoryError> {
        // Version zéro : le panier n'existe pas encore. Deux onglets qui
        // ajoutent une première ligne en même temps se disputent la clé
        // primaire, et le perdant doit voir un conflit de concurrence — pas une
        // erreur base incompréhensible.
        if expected_version == 0 {
            sqlx::query(
                "INSERT INTO players__customisation_baskets (player_id, space_id, state, version)
                 VALUES ($1, $2, $3, 1)",
            )
            .bind(&basket.player_id)
            .bind(&basket.space_id)
            .bind(&basket.state)
            .execute(&self.pool)
            .await
            .map_err(|e| match est_conflit_de_cle(&e) {
                true => RepositoryError::ConcurrentWrite,
                false => RepositoryError::Database(e),
            })?;
            return Ok(1);
        }

        // `updated_at = now()` à chaque écriture : c'est lui qui porte la
        // péremption, et la fenêtre glisse donc sur l'activité réelle.
        let resultat = sqlx::query(
            "UPDATE players__customisation_baskets
                SET state = $2, version = version + 1, updated_at = now()
              WHERE player_id = $1 AND version = $3",
        )
        .bind(&basket.player_id)
        .bind(&basket.state)
        .bind(expected_version as i32)
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        // Zéro ligne touchée : soit la version a bougé sous nos pieds, soit le
        // panier a été supprimé entre-temps. Dans les deux cas, l'appelant
        // travaillait sur un état périmé.
        match resultat.rows_affected() {
            0 => Err(RepositoryError::ConcurrentWrite),
            _ => Ok(expected_version + 1),
        }
    }

    async fn delete(&self, player_id: &str) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM players__customisation_baskets WHERE player_id = $1")
            .bind(player_id)
            .execute(&self.pool)
            .await
            .map_err(RepositoryError::Database)?;
        Ok(())
    }
}
