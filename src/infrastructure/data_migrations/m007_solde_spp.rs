//! Carte 569 — remplir le solde de SPP projeté.
//!
//! La colonne `players_proj.spp_remaining` naît à zéro. Tout joueur qui a
//! gagné ou dépensé des SPP avant elle doit être rejoué.
//!
//! # Par le chemin nominal
//!
//! Elle appelle `recompute_spp_remaining`, la fonction que la projection
//! exécute à chaque gain ou dépense, qui lit `Player::spp_remaining` — la
//! seule définition du solde. Comme `m006`, elle ne recopie aucun calcul.
//!
//! # Rejouable sans risque
//!
//! Le solde est **reposé**, jamais incrémenté, et écrit dans la transaction de
//! sa marque.

use crate::app::players::io::repository::player_repository::recompute_spp_remaining;
use crate::infrastructure::data_migrations::DataMigration;
use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{Postgres, Transaction};

pub struct SoldeSpp;

#[async_trait]
impl DataMigration for SoldeSpp {
    fn nom(&self) -> &'static str {
        "569-solde-spp"
    }

    /// Rend le nombre de joueurs dont le solde est non nul.
    async fn executer(
        &self,
        _state: &AppState,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<usize, String> {
        let joueurs: Vec<String> =
            sqlx::query_scalar("SELECT player_id FROM players_proj ORDER BY player_id")
                .fetch_all(&mut **tx)
                .await
                .map_err(|e| format!("lecture des joueurs : {e}"))?;

        for player_id in &joueurs {
            recompute_spp_remaining(tx, player_id)
                .await
                .map_err(|e| format!("recalcul pour {player_id} : {e}"))?;
        }

        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM players_proj WHERE spp_remaining > 0")
            .fetch_one(&mut **tx)
            .await
            .map(|n| n as usize)
            .map_err(|e| format!("décompte : {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::events::PlayerDomainEvent;
    use crate::app::players::domain::match_impact::SppEarned;
    use crate::app::players::domain::player::{PlayerId, TeamId};
    use crate::app::players::io::repository::player_repository::PgPlayerRepository;
    use crate::app::players::io::repository::tests::test_player_repository::{
        sample_context, seed_player,
    };
    use crate::app::players::ports::IPlayerRepository;
    use crate::infrastructure::data_migrations::appliquer;

    /// Un joueur crédité avant la colonne : son solde est à zéro en
    /// projection, l'agrégat dit trois. La migration pose trois.
    #[sqlx::test]
    async fn un_joueur_credite_avant_la_colonne_recoit_son_solde(pool: sqlx::PgPool) {
        let repo = PgPlayerRepository::new(pool.clone());
        let team_id = TeamId("t-m007".into());
        let joueur = PlayerId("marqueur".into());
        seed_player(&repo, &joueur, &team_id).await;
        let event = PlayerDomainEvent::TouchdownScored {
            player_id: joueur.clone(),
            team_id: team_id.clone(),
            context: sample_context(),
            spp_earned: SppEarned::try_new(3).unwrap(),
        };
        repo.append(&joueur, &team_id, &event, 2).await.unwrap();
        // L'état d'une base d'avant la carte.
        sqlx::query("UPDATE players_proj SET spp_remaining = 0")
            .execute(&pool)
            .await
            .unwrap();

        let state = crate::compose(crate::config::AppConfig::for_tests(), pool.clone()).await;
        appliquer(&state, &pool, vec![Box::new(SoldeSpp)])
            .await
            .unwrap();

        let solde: i32 = sqlx::query_scalar(
            "SELECT spp_remaining FROM players_proj WHERE player_id = 'marqueur'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(solde, 3);
    }
}
