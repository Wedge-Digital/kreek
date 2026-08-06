use crate::app::teams::domain::team::{Team, TeamDomainEvent};
use crate::app::teams::domain::treasury::TreasuryMovement;
use crate::app::teams::ports::{
    ITeamRepository, MyTeamRow, RepositoryError, TeamCardRow, TeamEnrollmentRow,
};
use crate::common::services::event_bus::event_bus::EventBus;
use async_trait::async_trait;
use sqlx::{PgPool, Row};

pub struct TeamRepository {
    pool: PgPool,
    /// Bus interne du BC. Le passer au constructeur plutôt qu'à l'append rend
    /// impossible un repository qui n'publierait pas — donc un chemin de
    /// mutation que le recalcul de TV ne verrait jamais.
    event_bus: EventBus,
}

impl TeamRepository {
    pub fn new(pool: PgPool, event_bus: EventBus) -> Self {
        Self { pool, event_bus }
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
            TeamDomainEvent::MatchReportingCancelled { .. } => {
                sqlx::query(
                    "UPDATE team_proj SET game_phase = 'ReadyToPlay', updated_at = now() WHERE team_id = $1",
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
            // La TV projetée suit l'agrégat dans la même transaction que
            // l'append — sans quoi une projection désynchronisée afficherait
            // une valeur périmée sur la fiche équipe.
            TeamDomainEvent::TeamValueRecomputed { value } => {
                sqlx::query(
                    "UPDATE team_proj SET team_value = $2, updated_at = now() WHERE team_id = $1",
                )
                .bind(team_id)
                .bind(value.0 as i32)
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

    /// Hydrate l'agrégat depuis la transaction courante.
    ///
    /// Le solde ne peut pas être reconstitué en SQL sans réécrire l'écrêtage à
    /// zéro qui vit déjà dans `TreasuryMovement::debit` — deux implémentations
    /// d'une même règle finissent toujours par diverger. On paie donc une
    /// relecture des événements, dans la transaction pour rester cohérent avec
    /// l'écriture qui suit.
    async fn hydrate_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        team_id: &str,
    ) -> Result<Team, RepositoryError> {
        let rows = sqlx::query(
            "SELECT payload FROM team_event_store WHERE team_id = $1 ORDER BY version ASC",
        )
        .bind(team_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(RepositoryError::Database)?;

        let events: Vec<TeamDomainEvent> = rows
            .iter()
            .map(|r| serde_json::from_value(r.get("payload")))
            .collect::<Result<_, _>>()
            .map_err(RepositoryError::Deserialization)?;

        // Aucun événement : l'équipe naît avec cet append, son solde de départ
        // est zéro — ce que donne `Team::default()`.
        Ok(Team::hydrate(&events).unwrap_or_default())
    }

    /// Écrit N événements à versions croissantes, dans la transaction fournie :
    /// event store, projection et grand livre pour chacun.
    ///
    /// L'agrégat est hydraté **une fois**, puis avancé en mémoire d'un
    /// événement à l'autre. C'est ce qui permet à chaque mouvement de trésorerie
    /// de partir du solde laissé par le précédent, sans jamais recalculer en
    /// SQL — le point qui avait fait mettre ce calcul dans le domaine.
    ///
    /// `append` et `append_batch` passent tous deux par ici : un lot d'un seul
    /// événement suit exactement le même chemin qu'un append unitaire.
    async fn write_events_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        team_id: &str,
        events: &[TeamDomainEvent],
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        let mut team = self.hydrate_in_tx(tx, team_id).await?;
        let mut version = expected_version;

        for event in events {
            version += 1;
            let movement = team.treasury_movement(event);
            self.insert_event_in_tx(tx, team_id, event, version, movement.is_some())
                .await?;
            self.update_projection_in_tx(tx, team_id, event, version)
                .await?;
            if let Some(movement) = movement {
                self.insert_ledger_entry_in_tx(tx, team_id, version, &movement)
                    .await?;
            }
            // Avance l'état en mémoire : le mouvement suivant partira du solde
            // que celui-ci vient de laisser.
            team = team.apply(event);
        }

        Ok(version)
    }

    async fn insert_event_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        team_id: &str,
        event: &TeamDomainEvent,
        version: u64,
        moves_treasury: bool,
    ) -> Result<(), RepositoryError> {
        let payload = serde_json::to_value(event).map_err(RepositoryError::Serialization)?;
        // Le tag est **dérivé** du mouvement, jamais posé événement par
        // événement : ce serait recréer l'endroit qu'on peut oublier.
        let tags = if moves_treasury {
            serde_json::json!(["treasury"])
        } else {
            serde_json::json!([])
        };

        sqlx::query(
            "INSERT INTO team_event_store (team_id, event_type, event_version, payload, version, tags)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(team_id)
        .bind(event.type_name())
        .bind(event.schema_version())
        .bind(&payload)
        .bind(version as i64)
        .bind(&tags)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db) = e {
                if db.constraint() == Some("team_event_store_version") {
                    return RepositoryError::ConcurrentWrite;
                }
            }
            RepositoryError::Database(e)
        })?;
        Ok(())
    }

    /// Une ligne de grand livre par mouvement, dans la transaction de l'append.
    /// `ON CONFLICT DO NOTHING` sur `(team_id, event_version)` : rejouer un
    /// événement ne duplique pas sa ligne.
    async fn insert_ledger_entry_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        team_id: &str,
        version: u64,
        movement: &TreasuryMovement,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO teams__treasury_ledger
                 (team_id, event_version, direction, amount_kpo, reason, balance_after_kpo)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (team_id, event_version) DO NOTHING",
        )
        .bind(team_id)
        .bind(version as i64)
        .bind(movement.direction.as_str())
        .bind(movement.amount.0 as i32)
        .bind(movement.reason.as_str())
        .bind(movement.balance_after.0 as i32)
        .execute(&mut **tx)
        .await
        .map_err(RepositoryError::Database)?;
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
        self.append_batch(team_id, std::slice::from_ref(event), expected_version)
            .await
    }

    /// Applique N événements **atomiquement** : soit tout est écrit, soit rien.
    ///
    /// La validation d'une phase applique tout le panier d'un coup — N
    /// recrutements ou N renvois, plus la transition de phase. En N appels à
    /// `append`, une panne au milieu laisserait l'équipe à moitié servie et sa
    /// phase non validée.
    ///
    /// Un lot vide de mutations reste légitime : terminer sa phase sans rien
    /// acheter, c'est un lot d'un seul événement de transition.
    async fn append_batch(
        &self,
        team_id: &str,
        events: &[TeamDomainEvent],
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        if events.is_empty() {
            return Ok(expected_version);
        }

        let mut tx = self.pool.begin().await.map_err(RepositoryError::Database)?;
        let derniere_version = self
            .write_events_in_tx(&mut tx, team_id, events, expected_version)
            .await?;
        tx.commit().await.map_err(RepositoryError::Database)?;

        // Publiés après le commit, dans l'ordre, et depuis le repository plutôt
        // que depuis chaque use case : deux des quatre chemins vers
        // `ReadyToPlay` passent par des listeners. C'est le seul point qui les
        // couvre tous. Déviation assumée du patron de `players` et
        // `match_report`.
        for event in events {
            let _ = self.event_bus.send(event.to_enveloppe(team_id));
        }

        Ok(derniere_version)
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

    async fn find_by_coach_and_space(
        &self,
        coach_id: &str,
        space_id: &str,
    ) -> Result<Vec<MyTeamRow>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            team_id: String,
            team_name: String,
            roster_name: String,
            logo_url: Option<String>,
            status: String,
            game_phase: Option<String>,
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT team_id, team_name, roster_name, logo_url, status, game_phase
             FROM team_proj
             WHERE coach_id = $1 AND space_id = $2
             ORDER BY updated_at DESC",
        )
        .bind(coach_id)
        .bind(space_id)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(rows
            .into_iter()
            .map(|r| MyTeamRow {
                team_id: r.team_id,
                team_name: r.team_name,
                roster_name: r.roster_name,
                logo_url: r.logo_url,
                status: r.status,
                game_phase: r.game_phase,
            })
            .collect())
    }
}

// ── Tests d'intégration ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::app::teams::domain::value_objects::{
        DedicatedFans, IncidentType, Kpo, RosterName, StaffQuantity, StaffType, TeamName,
    };
    use crate::common::services::event_bus::event_bus::new_bus;
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
        use crate::app::shared_kernel::bloodbowl::staff_counts::{
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
        let repo = TeamRepository::new(pool, new_bus());
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
        let repo = TeamRepository::new(pool, new_bus());
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
        let repo = TeamRepository::new(pool, new_bus());
        let team_id = ulid::Ulid::new().to_string();
        let mr_id = crate::app::shared_kernel::bloodbowl::ids::MatchReportId::try_new(
            "00000000000000000000000007",
        )
        .unwrap();

        repo.append(&team_id, &created_event(&team_id), 0)
            .await
            .unwrap();
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
            &TeamDomainEvent::MatchReportingStarted {
                match_report_id: mr_id,
            },
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
        assert_eq!(
            projected_phase(&repo, &team_id).await,
            Some("PlayerImprovement".to_string())
        );

        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        let reverted = team.revert_post_match_sequence(mr_id).unwrap();
        repo.append_batch(&team_id, &reverted, 4).await.unwrap();

        assert_eq!(
            projected_phase(&repo, &team_id).await,
            Some("MatchReporting".to_string()),
            "la projection doit suivre l'agrégat"
        );
        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        assert_eq!(
            team.treasury.0, 1000,
            "le gain doit être retiré de l'agrégat"
        );

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

    /// Même garde-fou que le test précédent, pour l'annulation : un arm de
    /// projection oublié laisserait la projection sur « MatchReporting » —
    /// l'équipe redeviendrait jouable côté agrégat mais afficherait encore un
    /// match en cours.
    #[tokio::test]
    async fn annulation_de_saisie_met_a_jour_la_phase_dans_la_projection() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool, new_bus());
        let team_id = ulid::Ulid::new().to_string();
        let mr_id = crate::app::shared_kernel::bloodbowl::ids::MatchReportId::try_new(
            "00000000000000000000000007",
        )
        .unwrap();

        repo.append(&team_id, &created_event(&team_id), 0)
            .await
            .unwrap();
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
            &TeamDomainEvent::MatchReportingStarted {
                match_report_id: mr_id,
            },
            2,
        )
        .await
        .unwrap();
        assert_eq!(
            projected_phase(&repo, &team_id).await,
            Some("MatchReporting".to_string())
        );

        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        let cancelled = team.cancel_match_reporting(mr_id).unwrap();
        repo.append(&team_id, &cancelled, 3).await.unwrap();

        assert_eq!(
            projected_phase(&repo, &team_id).await,
            Some("ReadyToPlay".to_string()),
            "la projection doit suivre l'agrégat"
        );

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

    /// Le grand livre suit l'agrégat pas à pas : chaque mouvement porte le
    /// montant **effectif** et le solde qui en résulte, et le dernier solde du
    /// livre est celui de l'équipe.
    #[tokio::test]
    async fn le_grand_livre_suit_la_tresorerie_de_l_agregat() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool.clone(), new_bus());
        let team_id = ulid::Ulid::new().to_string();

        // 1000 de dotation, puis une bourde de 1200 : on ne perd que 1000,
        // le solde tombe à zéro sans jamais passer en négatif.
        repo.append(&team_id, &created_event(&team_id), 0)
            .await
            .unwrap();
        let bourde = TeamDomainEvent::CostlyMistakesApplied {
            roll: 6,
            incident: IncidentType::None,
            gp_lost: Kpo(1200),
        };
        repo.append(&team_id, &bourde, 1).await.unwrap();

        let lignes: Vec<(String, i32, String, i32)> = sqlx::query_as(
            "SELECT direction, amount_kpo, reason, balance_after_kpo
             FROM teams__treasury_ledger WHERE team_id = $1 ORDER BY event_version",
        )
        .bind(&team_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(lignes.len(), 2, "dotation puis bourde");
        assert_eq!(
            lignes[0],
            ("Credit".into(), 1000, "InitialEndowment".into(), 1000)
        );
        assert_eq!(
            lignes[1],
            ("Debit".into(), 1000, "CostlyMistake".into(), 0),
            "le montant enregistré est celui réellement retiré, pas les 1200 décidés"
        );

        let team = repo.find_by_id(&team_id).await.unwrap().unwrap();
        assert_eq!(
            team.treasury.0, lignes[1].3 as u32,
            "le dernier solde du livre est celui de l'agrégat"
        );
    }

    /// Un événement sans effet sur la trésorerie ne laisse aucune ligne, et
    /// n'est pas tagué.
    #[tokio::test]
    async fn un_evenement_sans_mouvement_ne_touche_ni_livre_ni_tag() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool.clone(), new_bus());
        let team_id = ulid::Ulid::new().to_string();

        repo.append(&team_id, &created_event(&team_id), 0)
            .await
            .unwrap();
        repo.append(&team_id, &TeamDomainEvent::TeamDismissed, 1)
            .await
            .unwrap();

        let tags: Vec<(serde_json::Value,)> =
            sqlx::query_as("SELECT tags FROM team_event_store WHERE team_id = $1 ORDER BY version")
                .bind(&team_id)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(tags[0].0, serde_json::json!(["treasury"]));
        assert_eq!(tags[1].0, serde_json::json!([]));

        let n: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM teams__treasury_ledger WHERE team_id = $1")
                .bind(&team_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(n.0, 1, "seule la dotation initiale a laissé une ligne");
    }

    // ── append_batch (carte 256) ─────────────────────────────────────────

    fn achat_staff(cout: u32) -> TeamDomainEvent {
        TeamDomainEvent::StaffBought {
            staff_type: StaffType::Assistant,
            quantity: StaffQuantity::try_new(1).unwrap(),
            cost_kpo: Kpo(cout),
        }
    }

    #[tokio::test]
    async fn append_batch_ecrit_les_n_evenements_a_versions_consecutives() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool.clone(), new_bus());
        let team_id = ulid::Ulid::new().to_string();

        let lot = vec![
            created_event(&team_id),
            achat_staff(10),
            TeamDomainEvent::TeamDismissed,
        ];
        let derniere = repo.append_batch(&team_id, &lot, 0).await.unwrap();
        assert_eq!(derniere, 3, "la version du dernier événement du lot");

        let versions: Vec<(i64,)> = sqlx::query_as(
            "SELECT version FROM team_event_store WHERE team_id = $1 ORDER BY version",
        )
        .bind(&team_id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            versions.iter().map(|v| v.0).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );

        // Le solde du grand livre suit le lot d'un événement à l'autre :
        // 1000 de dotation, puis 10 dépensés.
        let soldes: Vec<(i32,)> = sqlx::query_as(
            "SELECT balance_after_kpo FROM teams__treasury_ledger
             WHERE team_id = $1 ORDER BY event_version",
        )
        .bind(&team_id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            soldes.iter().map(|v| v.0).collect::<Vec<_>>(),
            vec![1000, 990]
        );
    }

    /// L'atomicité est le point de la carte : un conflit de version sur le
    /// deuxième événement ne doit laisser **aucun** des trois.
    #[tokio::test]
    async fn append_batch_ne_laisse_rien_si_un_evenement_entre_en_conflit() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool.clone(), new_bus());
        let team_id = ulid::Ulid::new().to_string();

        // Version 2 déjà prise par un append antérieur.
        repo.append(&team_id, &created_event(&team_id), 0)
            .await
            .unwrap();
        repo.append(&team_id, &TeamDomainEvent::TeamDismissed, 1)
            .await
            .unwrap();

        // Le lot repart de la version 1 : son deuxième événement voudra la 3,
        // mais son premier tombera sur la 2, déjà occupée.
        let lot = vec![achat_staff(10), achat_staff(20), achat_staff(30)];
        let resultat = repo.append_batch(&team_id, &lot, 1).await;

        assert!(
            matches!(resultat, Err(RepositoryError::ConcurrentWrite)),
            "le conflit doit être détecté"
        );

        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM team_event_store WHERE team_id = $1")
            .bind(&team_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n.0, 2, "aucun événement du lot n'a été écrit");

        let achats: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM teams__treasury_ledger
             WHERE team_id = $1 AND reason = 'StaffPurchase'",
        )
        .bind(&team_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(achats.0, 0, "ni grand livre partiel");
    }

    /// Terminer une phase sans rien acheter est légitime.
    #[tokio::test]
    async fn append_batch_accepte_un_lot_d_un_seul_evenement() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool.clone(), new_bus());
        let team_id = ulid::Ulid::new().to_string();

        repo.append(&team_id, &created_event(&team_id), 0)
            .await
            .unwrap();
        let v = repo
            .append_batch(&team_id, &[TeamDomainEvent::RecruitmentPhaseValidated], 1)
            .await
            .unwrap();

        assert_eq!(v, 2);
    }

    #[tokio::test]
    async fn find_by_coach_and_space_liste_toutes_les_equipes_tous_statuts() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let repo = TeamRepository::new(pool, new_bus());
        let team_a = ulid::Ulid::new().to_string();
        let team_b = ulid::Ulid::new().to_string();
        let team_c = ulid::Ulid::new().to_string();

        repo.append(&team_a, &created_event(&team_a), 0)
            .await
            .unwrap();
        repo.append(
            &team_a,
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

        repo.append(&team_b, &created_event(&team_b), 0)
            .await
            .unwrap();
        repo.append(&team_b, &TeamDomainEvent::TeamDismissed, 1)
            .await
            .unwrap();

        // Équipe d'un autre space — ne doit pas apparaître dans la requête ci-dessous.
        use crate::app::shared_kernel::bloodbowl::staff_counts::{
            ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
        };
        let autre_space = TeamDomainEvent::TeamCreated {
            team_id: TeamId::try_new(&team_c).unwrap(),
            space_id: SpaceId::try_new("00000000000000000000000099").unwrap(),
            competition_id: CompetitionId::try_new("00000000000000000000000002").unwrap(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: SeasonId::try_new("00000000000000000000000003").unwrap(),
            season_name: "Saison 2025".to_string(),
            name: TeamName::try_new("Autre Space FC").unwrap(),
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
        };
        repo.append(&team_c, &autre_space, 0).await.unwrap();

        let coach_id = "00000000000000000000000005";
        let space_id = "00000000000000000000000001";
        let rows = repo
            .find_by_coach_and_space(coach_id, space_id)
            .await
            .unwrap();

        let row_a = rows
            .iter()
            .find(|r| r.team_id == team_a)
            .expect("team_a doit apparaître");
        assert_eq!(row_a.status, "Enrolled");
        assert_eq!(row_a.game_phase.as_deref(), Some("ReadyToPlay"));

        let row_b = rows
            .iter()
            .find(|r| r.team_id == team_b)
            .expect("team_b doit apparaître");
        assert_eq!(row_b.status, "Dismissed");

        assert!(
            !rows.iter().any(|r| r.team_id == team_c),
            "une équipe d'un autre space ne doit pas apparaître"
        );

        for id in [&team_a, &team_b, &team_c] {
            sqlx::query("DELETE FROM team_event_store WHERE team_id = $1")
                .bind(id)
                .execute(&repo.pool)
                .await
                .ok();
            sqlx::query("DELETE FROM team_proj WHERE team_id = $1")
                .bind(id)
                .execute(&repo.pool)
                .await
                .ok();
        }
    }
}
