//! Carte 664 — recalculer une fois toutes les valeurs d'équipe.
//!
//! # Pourquoi elle existe
//!
//! Jusqu'à la carte 664, un recalcul qui perdait la course contre un autre —
//! deux déclencheurs qui se chevauchent sur la même équipe — échouait sur
//! `ConcurrentWrite`, et rien ne le refaisait. Des valeurs d'équipe peuvent
//! donc être restées fausses en production depuis un recrutement, un renvoi ou
//! un jet d'erreurs coûteuses. Le use case retente désormais ; il ne répare
//! rien rétroactivement. C'est l'objet de cette migration.
//!
//! # Toutes les équipes, et pas seulement les touchées
//!
//! Les touchées ne se listent pas : le journal qui les nommait est celui du
//! conteneur, et un redéploiement l'emporte. Recalculer toutes les équipes est
//! sans risque : la valeur d'équipe est **une somme recalculée, jamais une
//! accumulation de deltas** — une valeur juste le reste. Le coût est un
//! `TeamValueRecomputed` de plus dans l'historique de chaque équipe.
//!
//! # Une copie de `m002`, sans sa garde
//!
//! Le corps est celui de `m002_recalcul_valeurs_equipe` (carte 388). Sa garde
//! `LOW_COST_LINEMEN` n'est pas reprise : elle protégeait le **premier**
//! recalcul sous la règle, et `m002` ne s'applique qu'après elle dans le
//! registre — une base qui atteint `m008` a passé cette garde.
//!
//! # Elle n'est pas atomique avec sa marque, et c'est assumé
//!
//! Elle passe par `recompute_team_value_use_case`, qui ouvre sa propre
//! transaction par équipe — c'est ce qui en fait le mécanisme *nominal* plutôt
//! qu'une écriture parallèle à réinventer. La règle d'atomicité de la carte 386
//! y perd donc son effet.
//!
//! Sans conséquence ici : l'événement appendu porte une valeur **absolue**, pas
//! un delta. Une interruption au milieu laisse des équipes recalculées et
//! d'autres non ; le rejeu au démarrage suivant repasse sur toutes, et
//! recalculer une valeur déjà juste la laisse juste. C'est précisément ce qu'un
//! delta ne permettrait pas.

use crate::app::teams::use_cases::recompute_team_value_use_case;
use crate::infrastructure::data_migrations::DataMigration;
use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{Postgres, Row, Transaction};

pub struct RecomputeTeamValues;

#[async_trait]
impl DataMigration for RecomputeTeamValues {
    fn nom(&self) -> &'static str {
        "664-recalcul-valeurs-equipe"
    }

    async fn executer(
        &self,
        state: &AppState,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<usize, String> {
        let equipes: Vec<String> = sqlx::query("SELECT team_id FROM team_proj ORDER BY team_id")
            .fetch_all(&mut **tx)
            .await
            .map_err(|e| format!("lecture des équipes : {e}"))?
            .into_iter()
            .map(|r| r.get("team_id"))
            .collect();

        for team_id in &equipes {
            recompute_team_value_use_case::execute(
                team_id,
                state.teams.team_repository.as_ref(),
                state.teams.squad_port.as_ref(),
                state.teams.roster_catalog_port.as_ref(),
                state.teams.journeyman_type_port.as_ref(),
            )
            .await
            .map_err(|e| format!("recalcul de {team_id} : {e:?}"))?;
        }
        Ok(equipes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::team::TeamDomainEvent;
    use crate::app::teams::domain::value_objects::Kpo;
    use crate::app::teams::io::web::tests::fixtures::equipe_semee;
    use crate::infrastructure::data_migrations::appliquer;

    /// Une valeur visiblement fausse, posée comme l'aurait laissée un
    /// recalcul perdu.
    const STALE_VALUE: u32 = 9995;

    async fn state(pool: sqlx::PgPool) -> AppState {
        crate::compose(crate::config::AppConfig::for_tests(), pool).await
    }

    /// Une équipe semée par le dépôt, puis figée à une valeur fausse.
    async fn team_with_stale_value(pool: &sqlx::PgPool, state: &AppState) -> String {
        let (_, team_id) = equipe_semee(pool).await;
        let stale = TeamDomainEvent::TeamValueRecomputed {
            value: Kpo(STALE_VALUE),
        };
        state
            .teams
            .team_repository
            .append(&team_id, &stale, 1)
            .await
            .expect("valeur fausse posée");
        team_id
    }

    async fn projected_value(pool: &sqlx::PgPool, team_id: &str) -> i32 {
        sqlx::query_scalar("SELECT team_value FROM team_proj WHERE team_id = $1")
            .bind(team_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn event_count(pool: &sqlx::PgPool, team_id: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM team_event_store WHERE team_id = $1")
            .bind(team_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn run(state: &AppState, pool: &sqlx::PgPool) {
        appliquer(state, pool, vec![Box::new(RecomputeTeamValues)])
            .await
            .expect("la migration passe");
    }

    /// Le rattrapage : la valeur fausse est remplacée par la valeur recalculée,
    /// la même que donne le use case sur l'état courant.
    #[sqlx::test]
    async fn a_stale_team_value_is_recomputed(pool: sqlx::PgPool) {
        let state = state(pool.clone()).await;
        let team_id = team_with_stale_value(&pool, &state).await;
        assert_eq!(projected_value(&pool, &team_id).await, STALE_VALUE as i32);

        run(&state, &pool).await;

        let after = projected_value(&pool, &team_id).await;
        assert_ne!(after, STALE_VALUE as i32, "la valeur fausse doit partir");
        assert_eq!(event_count(&pool, &team_id).await, 3, "un recalcul appendu");

        // La référence : un recalcul nominal de plus ne change plus rien.
        recompute_team_value_use_case::execute(
            &team_id,
            state.teams.team_repository.as_ref(),
            state.teams.squad_port.as_ref(),
            state.teams.roster_catalog_port.as_ref(),
            state.teams.journeyman_type_port.as_ref(),
        )
        .await
        .unwrap();
        assert_eq!(projected_value(&pool, &team_id).await, after);
    }

    /// Marquée appliquée, elle ne repasse pas : la seconde exécution n'appende
    /// rien et laisse la marque unique.
    #[sqlx::test]
    async fn a_second_run_does_nothing(pool: sqlx::PgPool) {
        let state = state(pool.clone()).await;
        let team_id = team_with_stale_value(&pool, &state).await;

        run(&state, &pool).await;
        let events_after_first = event_count(&pool, &team_id).await;
        run(&state, &pool).await;

        assert_eq!(event_count(&pool, &team_id).await, events_after_first);
        let marks: Vec<String> = sqlx::query_scalar("SELECT name FROM applied_data_migrations")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(marks, vec!["664-recalcul-valeurs-equipe"]);
    }
}
