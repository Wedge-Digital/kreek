//! Le canal temps réel demande à `teams` qui coache les équipes d'un match
//! (carte 645, épic E23).
//!
//! **Seul fichier de la chaîne à importer le BC source.** `src/web/realtime/`
//! ne connaît que `IMatchAudiencePort`.
//!
//! # Une requête de projection, pas deux rejeux
//!
//! Le routeur pose la question à chaque fait qui fait un toast — chaque action
//! saisie. `find_by_id` rejouerait les deux flux d'équipe à chaque fois ;
//! `find_coach_ids_in_space` lit `team_proj` en une requête, borne d'espace
//! comprise (décision 6).

use crate::app::shared_kernel::identity::ids::UserId;
use crate::app::teams::ports::ITeamRepository;
use crate::web::ports::IMatchAudiencePort;
use async_trait::async_trait;
use std::sync::Arc;

pub struct MatchAudienceAdapter {
    team_repo: Arc<dyn ITeamRepository>,
}

impl MatchAudienceAdapter {
    pub fn new(team_repo: Arc<dyn ITeamRepository>) -> Self {
        Self { team_repo }
    }
}

#[async_trait]
impl IMatchAudiencePort for MatchAudienceAdapter {
    async fn coaches_of_match(
        &self,
        space_id: &str,
        home_team_id: &str,
        away_team_id: &str,
    ) -> Vec<UserId> {
        let teams = [home_team_id, away_team_id];
        match self
            .team_repo
            .find_coach_ids_in_space(&teams, space_id)
            .await
        {
            Ok(coach_ids) => parse_coach_ids(coach_ids),
            // Échoue fermé : aucun toast plutôt qu'un toast mal adressé.
            Err(e) => {
                tracing::error!(space_id, error = %e, "coachs du match illisibles");
                Vec::new()
            }
        }
    }
}

/// Un identifiant illisible est écarté et signalé : il ne désignerait aucune
/// connexion du registre. Un coach des deux équipes n'est compté qu'une fois.
fn parse_coach_ids(coach_ids: Vec<String>) -> Vec<UserId> {
    let mut coaches: Vec<UserId> = Vec::new();
    for raw in coach_ids {
        match UserId::try_new(&raw) {
            Ok(id) if !coaches.contains(&id) => coaches.push(id),
            Ok(_) => {}
            Err(e) => tracing::warn!(coach_id = %raw, error = %e, "coach_id illisible"),
        }
    }
    coaches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::services::event_bus::event_bus::new_bus;

    async fn insert_team(pool: &sqlx::PgPool, space_id: &str, coach_id: &UserId) -> String {
        let team_id = UserId::new().to_string();
        sqlx::query(
            "INSERT INTO team_proj
                (team_id, space_id, team_name, coach_name, roster_name, status, team_value, coach_id)
             VALUES ($1, $2, 'Équipe de test', '', 'Granitiers', 'Enrolled', 0, $3)",
        )
        .bind(&team_id)
        .bind(space_id)
        .bind(coach_id.to_string())
        .execute(pool)
        .await
        .expect("équipe de test");
        team_id
    }

    fn adapter(pool: sqlx::PgPool) -> MatchAudienceAdapter {
        MatchAudienceAdapter::new(Arc::new(
            crate::app::teams::io::repository::team_repository::TeamRepository::new(
                pool,
                new_bus(),
            ),
        ))
    }

    #[sqlx::test]
    async fn both_coaches_of_the_space_are_returned(pool: sqlx::PgPool) {
        let space = UserId::new().to_string();
        let (home_coach, away_coach) = (UserId::new(), UserId::new());
        let home = insert_team(&pool, &space, &home_coach).await;
        let away = insert_team(&pool, &space, &away_coach).await;

        let coaches = adapter(pool).coaches_of_match(&space, &home, &away).await;

        assert_eq!(coaches.len(), 2);
        assert!(coaches.contains(&home_coach) && coaches.contains(&away_coach));
    }

    #[sqlx::test]
    async fn a_team_of_another_space_does_not_return_its_coach(pool: sqlx::PgPool) {
        let (space, other_space) = (UserId::new().to_string(), UserId::new().to_string());
        let (home_coach, stranger) = (UserId::new(), UserId::new());
        let home = insert_team(&pool, &space, &home_coach).await;
        let away = insert_team(&pool, &other_space, &stranger).await;

        let coaches = adapter(pool).coaches_of_match(&space, &home, &away).await;

        assert_eq!(coaches, vec![home_coach]);
    }

    #[test]
    fn an_unreadable_coach_id_is_dropped_and_duplicates_merged() {
        let coach = UserId::new();
        let coaches = parse_coach_ids(vec!["c".into(), coach.to_string(), coach.to_string()]);
        assert_eq!(coaches, vec![coach]);
    }
}
