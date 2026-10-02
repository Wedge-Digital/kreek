//! Carte 568 — remplir le compteur de blessures persistantes.
//!
//! La colonne `players_proj.persistent_injuries` naît à zéro. Les joueurs qui
//! ont subi une blessure sérieuse avant elle n'ont jamais vu passer l'écriture
//! qui la tient : il faut rejouer chacun.
//!
//! # Par le chemin nominal
//!
//! Elle appelle `recompute_persistent_injuries`, la fonction même que la
//! projection exécute à chaque blessure. Il n'existe donc qu'une façon de
//! calculer ce compteur : une copie ici aurait pu diverger sans bruit.
//!
//! # Rejouable sans risque
//!
//! Le compteur est **reposé**, jamais incrémenté. Interrompue puis rejouée, la
//! migration recalcule une valeur déjà juste et la laisse juste — et elle écrit
//! dans la transaction de sa marque.

use crate::app::players::io::repository::player_repository::recompute_persistent_injuries;
use crate::infrastructure::data_migrations::DataMigration;
use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{Postgres, Transaction};

pub struct BlessuresPersistantes;

#[async_trait]
impl DataMigration for BlessuresPersistantes {
    fn nom(&self) -> &'static str {
        "568-blessures-persistantes"
    }

    /// Rend le nombre de joueurs qui portent au moins une blessure persistante.
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
            recompute_persistent_injuries(tx, player_id)
                .await
                .map_err(|e| format!("recalcul pour {player_id} : {e}"))?;
        }

        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM players_proj WHERE persistent_injuries > 0",
        )
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
    use crate::app::players::domain::match_impact::InjuryType;
    use crate::app::players::domain::player::{PlayerId, TeamId};
    use crate::app::players::io::repository::player_repository::PgPlayerRepository;
    use crate::app::players::io::repository::tests::test_player_repository::{
        sample_context, seed_player,
    };
    use crate::app::players::ports::IPlayerRepository;
    use crate::infrastructure::data_migrations::appliquer;

    async fn compteur(pool: &sqlx::PgPool, player_id: &str) -> i16 {
        sqlx::query_scalar("SELECT persistent_injuries FROM players_proj WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// Un joueur blessé avant la colonne : son compteur est à zéro en
    /// projection, l'agrégat dit un. La migration pose un.
    #[sqlx::test]
    async fn un_joueur_blesse_avant_la_colonne_recoit_son_compteur(pool: sqlx::PgPool) {
        let repo = PgPlayerRepository::new(pool.clone());
        let team_id = TeamId("t-m006".into());
        let joueur = PlayerId("blesse".into());
        seed_player(&repo, &joueur, &team_id).await;
        let event = PlayerDomainEvent::InjurySustained {
            player_id: joueur.clone(),
            team_id: team_id.clone(),
            context: sample_context(),
            injury_type: InjuryType::BlessureSerieuse,
        };
        repo.append(&joueur, &team_id, &event, 2).await.unwrap();
        // L'état d'une base d'avant la carte.
        sqlx::query("UPDATE players_proj SET persistent_injuries = 0")
            .execute(&pool)
            .await
            .unwrap();

        let state = crate::compose(crate::config::AppConfig::for_tests(), pool.clone()).await;
        appliquer(&state, &pool, vec![Box::new(BlessuresPersistantes)])
            .await
            .unwrap();

        assert_eq!(compteur(&pool, "blesse").await, 1);
    }
}
