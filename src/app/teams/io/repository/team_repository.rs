use crate::app::teams::domain::team::{Team, TeamDomainEvent};
use crate::app::teams::ports::{ITeamRepository, RepositoryError, TeamCardRow, TeamEnrollmentRow};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

pub struct TeamRepository {
    pool: PgPool,
}

impl TeamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn update_projection_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        team_id: &str,
        event: &TeamDomainEvent,
        _version: u64,
    ) -> Result<(), RepositoryError> {
        match event {
            TeamDomainEvent::TeamCreated {
                space_id,
                competition_id,
                season_id,
                name,
                logo_url,
                coach_id,
                coach_name,
                roster_name,
                treasury,
                ..
            } => {
                sqlx::query(
                    "INSERT INTO team_proj (team_id, space_id, competition_id, season_id, team_name, coach_id, coach_name, roster_name, logo_url, team_value, status, game_phase)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'PendingEnrollment', NULL)
                     ON CONFLICT (team_id) DO UPDATE SET
                       team_name = EXCLUDED.team_name,
                       coach_id = EXCLUDED.coach_id,
                       coach_name = EXCLUDED.coach_name,
                       roster_name = EXCLUDED.roster_name,
                       competition_id = EXCLUDED.competition_id,
                       season_id = EXCLUDED.season_id,
                       logo_url = EXCLUDED.logo_url,
                       team_value = EXCLUDED.team_value,
                       updated_at = now()",
                )
                .bind(team_id)
                .bind(space_id.to_string())
                .bind(competition_id.to_string())
                .bind(season_id.to_string())
                .bind(name.as_ref())
                .bind(coach_id.to_string())
                .bind(coach_name)
                .bind(roster_name.as_ref())
                .bind(logo_url)
                .bind(treasury.0 as i32)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            TeamDomainEvent::TeamEnrolled {
                competition_id,
                season_id,
                ..
            } => {
                sqlx::query(
                    "UPDATE team_proj
                     SET status = 'Enrolled', competition_id = $2, season_id = $3, game_phase = 'ReadyToPlay', updated_at = now()
                     WHERE team_id = $1",
                )
                .bind(team_id)
                .bind(competition_id.to_string())
                .bind(season_id.to_string())
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            TeamDomainEvent::MatchReportingStarted { .. } => {
                sqlx::query(
                    "UPDATE team_proj SET game_phase = 'MatchReporting', updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            TeamDomainEvent::TeamDismissed => {
                sqlx::query(
                    "UPDATE team_proj SET status = 'Dismissed', updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            TeamDomainEvent::TeamEnrollmentRejected { .. } => {
                sqlx::query(
                    "UPDATE team_proj SET status = 'Rejected', updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            TeamDomainEvent::PostMatchSequenceStarted { .. } => {
                sqlx::query(
                    "UPDATE team_proj SET game_phase = 'PlayerImprovement', updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            TeamDomainEvent::PlayerImprovementPhaseValidated => {
                sqlx::query(
                    "UPDATE team_proj SET game_phase = 'Recruitment', updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            TeamDomainEvent::RecruitmentPhaseValidated => {
                sqlx::query(
                    "UPDATE team_proj SET game_phase = 'Dismissals', updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            TeamDomainEvent::DismissalsPhaseValidated => {
                sqlx::query(
                    "UPDATE team_proj SET game_phase = 'ReadyToPlay', updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            // Retour en arrière : le rapport a été dépublié pour correction.
            // Sans cet arm, l'agrégat repasserait en MatchReporting pendant que
            // la projection resterait bloquée sur PlayerImprovement — le `_ =>`
            // ci-dessous compile sans broncher, c'est exactement le bug de la
            // carte 175.
            TeamDomainEvent::PostMatchSequenceReverted { .. } => {
                sqlx::query(
                    "UPDATE team_proj SET game_phase = 'MatchReporting', updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .execute(&mut **tx)
                .await
                .map_err(RepositoryError::Database)?;
            }
            // Autres événements (retraite temporaire, off-season, override admin) : pas encore
            // produits par aucun use case (cartes 39/40/43/46 à faire) — quand l'un d'eux sera
            // implémenté, ajouter ici l'arm correspondant, sous peine de reproduire le bug de
            // désynchronisation de team_proj.game_phase déjà rencontré (cf. carte 175).
            _ => {}
        }
        Ok(())
    }
}

#[async_trait]
impl ITeamRepository for TeamRepository {
    async fn append(
        &self,
        team_id: &str,
        event: &TeamDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        let new_version = expected_version + 1;
        let payload = serde_json::to_value(event).map_err(RepositoryError::Serialization)?;
        let event_type = event.type_name();
        let event_version = event.schema_version();

        let mut tx = self.pool.begin().await.map_err(RepositoryError::Database)?;

        sqlx::query(
            "INSERT INTO team_event_store (team_id, event_type, event_version, payload, version)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(team_id)
        .bind(event_type)
        .bind(event_version)
        .bind(&payload)
        .bind(new_version as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db) = e {
                if db.constraint() == Some("team_event_store_version") {
                    return RepositoryError::ConcurrentWrite;
                }
            }
            RepositoryError::Database(e)
        })?;

        self.update_projection_in_tx(&mut tx, team_id, event, new_version)
            .await?;

        tx.commit().await.map_err(RepositoryError::Database)?;
        Ok(new_version)
    }

    async fn find_by_id(&self, team_id: &str) -> Result<Option<Team>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT payload FROM team_event_store
             WHERE team_id = $1 ORDER BY version ASC",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        if rows.is_empty() {
            return Ok(None);
        }

        let events: Vec<TeamDomainEvent> = rows
            .iter()
            .map(|r| {
                let payload: serde_json::Value = r.get("payload");
                serde_json::from_value(payload)
            })
            .collect::<Result<_, _>>()
            .map_err(RepositoryError::Deserialization)?;

        Ok(Team::hydrate(&events))
    }

    async fn find_by_season_and_status(
        &self,
        season_id: &str,
        status: &str,
    ) -> Result<Vec<TeamEnrollmentRow>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            team_id: String,
            team_name: String,
            coach_name: String,
            roster_name: String,
            status: String,
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT team_id, team_name, coach_name, roster_name, status
             FROM team_proj
             WHERE season_id = $1 AND status = $2
             ORDER BY updated_at DESC",
        )
        .bind(season_id)
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(rows
            .into_iter()
            .map(|r| TeamEnrollmentRow {
                team_id: r.team_id,
                team_name: r.team_name,
                coach_name: r.coach_name,
                roster_name: r.roster_name,
                status: r.status,
            })
            .collect())
    }

    async fn find_enrolled_for_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<TeamCardRow>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            team_id: String,
            team_name: String,
            coach_id: String,
            coach_name: String,
            roster_name: String,
            logo_url: Option<String>,
            team_value: i32,
            game_phase: Option<String>,
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT team_id, team_name, coach_id, coach_name, roster_name, logo_url, team_value, game_phase
             FROM team_proj
             WHERE season_id = $1 AND status = 'Enrolled'
             ORDER BY team_name ASC",
        )
        .bind(season_id)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(rows
            .into_iter()
            .map(|r| TeamCardRow {
                team_id: r.team_id,
                team_name: r.team_name,
                coach_id: r.coach_id,
                coach_name: r.coach_name,
                roster_name: r.roster_name,
                logo_url: r.logo_url,
                team_value: r.team_value as u32,
                game_phase: r.game_phase,
            })
            .collect())
    }
}

// ── Tests d'intégration ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::common_types::{CoachId, CompetitionId, RosterId, SeasonId, SpaceId};
    use crate::app::shared_kernel::team::TeamId;
    use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .ok()
    }

    fn created_event(team_id: &str) -> TeamDomainEvent {
        use crate::app::shared_kernel::staff_counts::{
            ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
        };
        TeamDomainEvent::TeamCreated {
            team_id: TeamId::try_new(team_id).unwrap(),
            space_id: SpaceId::try_new("00000000000000000000000001").unwrap(),
            competition_id: CompetitionId::try_new("00000000000000000000000002").unwrap(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: SeasonId::try_new("00000000000000000000000003").unwrap(),
            season_name: "Saison 2025".to_string(),
            name: TeamName::try_new("Les Korrigans FC").unwrap(),
            logo_url: None,
            roster_id: RosterId::try_new("00000000000000000000000004").unwrap(),
            roster_name: RosterName::try_new("Elfes Sylvestres").unwrap(),
            coach_id: CoachId::try_new("00000000000000000000000005").unwrap(),
            coach_name: "Colonel Castor".to_string(),
            treasury: Kpo(1000),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(3),
            apothecaries: ApothecaryCount(1),
            assistants: AssistantCount(2),
            cheerleaders: CheerleaderCount(3),
        }
    }

    #[tokio::test]
    async fn append_and_find_by_id() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool);
        let team_id = ulid::Ulid::new().to_string();

        let event1 = created_event(&team_id);
        let v1 = repo.append(&team_id, &event1, 0).await.unwrap();
        assert_eq!(v1, 1);

        let event2 = TeamDomainEvent::TeamEnrolled {
            competition_id: CompetitionId::try_new("00000000000000000000000002").unwrap(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: SeasonId::try_new("00000000000000000000000003").unwrap(),
            season_name: "Saison 2025".to_string(),
        };
        let v2 = repo.append(&team_id, &event2, 1).await.unwrap();
        assert_eq!(v2, 2);

        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        assert_eq!(team.name.as_ref(), "Les Korrigans FC");
        assert_eq!(team.version, 2);
        assert_eq!(
            team.participation_status,
            crate::app::teams::domain::team::ParticipationStatus::Enrolled,
        );

        // Cleanup
        sqlx::query("DELETE FROM team_event_store WHERE team_id = $1")
            .bind(&team_id)
            .execute(&repo.pool)
            .await
            .ok();
    }

    #[tokio::test]
    async fn concurrent_write_is_rejected() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool);
        let team_id = ulid::Ulid::new().to_string();

        let event = created_event(&team_id);
        repo.append(&team_id, &event, 0).await.unwrap();

        // Deuxième append avec la même expected_version → ConcurrentWrite
        let event2 = TeamDomainEvent::TeamDismissed;
        let result = repo.append(&team_id, &event2, 0).await;
        assert!(matches!(result, Err(RepositoryError::ConcurrentWrite)));

        // Cleanup
        sqlx::query("DELETE FROM team_event_store WHERE team_id = $1")
            .bind(&team_id)
            .execute(&repo.pool)
            .await
            .ok();
    }
    /// Garde-fou contre le bug de la carte 175 : le `match` de projection se
    /// termine par un `_ => {}`, donc l'oubli d'un arm **compile sans
    /// broncher**. L'agrégat repasserait en MatchReporting pendant que la
    /// projection resterait bloquée sur PlayerImprovement, sans erreur nulle
    /// part.
    #[tokio::test]
    async fn revert_post_match_met_a_jour_la_phase_dans_la_projection() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool);
        let team_id = ulid::Ulid::new().to_string();
        let mr_id = crate::app::shared_kernel::common_types::MatchReportId::try_new(
            "00000000000000000000000007",
        )
        .unwrap();

        repo.append(&team_id, &created_event(&team_id), 0).await.unwrap();
        repo.append(
            &team_id,
            &TeamDomainEvent::TeamEnrolled {
                competition_id: CompetitionId::try_new("00000000000000000000000002").unwrap(),
                competition_name: "Ligue de Condate".to_string(),
                season_id: SeasonId::try_new("00000000000000000000000003").unwrap(),
                season_name: "Saison 2025".to_string(),
            },
            1,
        )
        .await
        .unwrap();
        repo.append(
            &team_id,
            &TeamDomainEvent::MatchReportingStarted { match_report_id: mr_id },
            2,
        )
        .await
        .unwrap();

        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        let started = team
            .start_post_match_sequence(
                crate::app::teams::domain::value_objects::MatchResult::Win,
                1,
                crate::app::teams::domain::value_objects::Kpo(150),
                vec![],
            )
            .unwrap();
        repo.append(&team_id, &started, 3).await.unwrap();
        assert_eq!(projected_phase(&repo, &team_id).await, Some("PlayerImprovement".to_string()));

        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        let reverted = team.revert_post_match_sequence(mr_id).unwrap();
        repo.append(&team_id, &reverted, 4).await.unwrap();

        assert_eq!(
            projected_phase(&repo, &team_id).await,
            Some("MatchReporting".to_string()),
            "la projection doit suivre l'agrégat"
        );
        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        assert_eq!(team.treasury.0, 1000, "le gain doit être retiré de l'agrégat");

        sqlx::query("DELETE FROM team_event_store WHERE team_id = $1")
            .bind(&team_id)
            .execute(&repo.pool)
            .await
            .ok();
        sqlx::query("DELETE FROM team_proj WHERE team_id = $1")
            .bind(&team_id)
            .execute(&repo.pool)
            .await
            .ok();
    }

    async fn projected_phase(repo: &TeamRepository, team_id: &str) -> Option<String> {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT game_phase FROM team_proj WHERE team_id = $1",
        )
        .bind(team_id)
        .fetch_one(&repo.pool)
        .await
        .unwrap()
    }
}
