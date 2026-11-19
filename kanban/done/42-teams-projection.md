# BC `teams` — Table de projection + requêtes de liste

**Priorité : haute**
**Dépend de :** `29-teams-repository.md`
**Contexte :** `teams` — infrastructure lecture

## Objectif

Maintenir une table de projection mise à jour à chaque `append()` (dans la même transaction), afin que les requêtes de liste (mes équipes, équipes inscrites) n'aient pas à rejouer les événements de chaque agrégat.

---

## Principe

La projection est un **dérivé rebuildable** de l'event store. Elle ne contient que les champs utiles aux listes et badges de statut. Pour l'état complet d'une équipe, on relit toujours l'event store (`find_by_id`).

Si la projection se désynchronise (bug, migration), elle peut être reconstruite intégralement en rejouant tous les événements de `team_event_store`.

---

## Conception

### Schéma SQL

```sql
-- migrations/YYYYMMDD_teams_projection.sql
CREATE TABLE teams_projection (
    team_id              TEXT        PRIMARY KEY,
    space_id             TEXT        NOT NULL,
    coach_id             TEXT        NOT NULL,
    name                 TEXT        NOT NULL,
    initials             TEXT        NOT NULL DEFAULT '',
    logo_url             TEXT,
    roster_name          TEXT        NOT NULL,
    participation_status TEXT        NOT NULL,  -- 'pending_enrollment' | 'enrolled' | 'dismissed'
    game_phase           TEXT,                  -- NULL si non inscrite
    dedicated_fans       SMALLINT    NOT NULL DEFAULT 0,
    treasury_kpo         INTEGER     NOT NULL DEFAULT 0,
    team_value_kpo       INTEGER     NOT NULL DEFAULT 0,  -- TV maintenue par update_projection_in_tx()
    competition_name     TEXT,                  -- dénormalisé depuis TeamEnrolled
    season_name          TEXT,                  -- idem
    version              BIGINT      NOT NULL,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX teams_proj_coach_space ON teams_projection (coach_id, space_id);
CREATE INDEX teams_proj_space       ON teams_projection (space_id);
```

### Mise à jour — `update_projection_in_tx()`

Appelée dans la transaction de `append()` — atomique avec l'insert dans l'event store.

```rust
async fn update_projection_in_tx(
    &self,
    tx:      &mut PgConnection,
    team_id: &TeamId,
    event:   &TeamDomainEvent,
    version: u64,
) -> Result<(), RepositoryError> {
    match event {
        TeamDomainEvent::TeamCreated { space_id, coach_id, name, roster_name, .. } => {
            sqlx::query!(
                "INSERT INTO teams_projection
                    (team_id, space_id, coach_id, name, roster_name,
                     participation_status, game_phase, version)
                 VALUES ($1, $2, $3, $4, $5, 'pending_enrollment', NULL, $6)
                 ON CONFLICT (team_id) DO NOTHING",
                team_id.as_str(), space_id, coach_id, name, roster_name, version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::TeamEnrolled { .. } => {
            sqlx::query!(
                "UPDATE teams_projection
                 SET participation_status = 'enrolled',
                     game_phase = 'ready_to_play', version = $2
                 WHERE team_id = $1",
                team_id.as_str(), version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::TeamDismissed => {
            sqlx::query!(
                "UPDATE teams_projection
                 SET participation_status = 'dismissed',
                     game_phase = NULL, version = $2
                 WHERE team_id = $1",
                team_id.as_str(), version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::MatchPlayedReceived { dedicated_fans_roll, treasury_income, result, .. } => {
            // fans et trésorerie calculés ici comme dans Team::apply()
            sqlx::query!(
                "UPDATE teams_projection
                 SET game_phase    = 'player_improvement',
                     dedicated_fans = $2,
                     treasury_kpo  = treasury_kpo + $3,
                     version       = $4
                 WHERE team_id = $1",
                team_id.as_str(),
                compute_fans(*dedicated_fans_roll, result) as i16,
                *treasury_income as i32,
                version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::PlayerImprovementPhaseValidated => {
            sqlx::query!("UPDATE teams_projection SET game_phase = 'recruitment', version = $2 WHERE team_id = $1",
                team_id.as_str(), version as i64).execute(tx).await?;
        }
        TeamDomainEvent::RecruitmentPhaseValidated => {
            sqlx::query!("UPDATE teams_projection SET game_phase = 'dismissals', version = $2 WHERE team_id = $1",
                team_id.as_str(), version as i64).execute(tx).await?;
        }
        TeamDomainEvent::DismissalsPhaseValidated => {
            sqlx::query!("UPDATE teams_projection SET game_phase = 'temporary_retirement', version = $2 WHERE team_id = $1",
                team_id.as_str(), version as i64).execute(tx).await?;
        }
        TeamDomainEvent::CostlyMistakesApplied { gp_lost, .. } => {
            sqlx::query!(
                "UPDATE teams_projection
                 SET game_phase   = 'ready_to_play',
                     treasury_kpo = GREATEST(treasury_kpo - $2, 0),
                     version      = $3
                 WHERE team_id = $1",
                team_id.as_str(), *gp_lost as i32, version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::PlayerRecruited { base_value_kpo, cost_kpo, .. } => {
            sqlx::query!(
                "UPDATE teams_projection
                 SET team_value_kpo = team_value_kpo + $2,
                     treasury_kpo   = GREATEST(treasury_kpo - $3, 0),
                     version        = $4
                 WHERE team_id = $1",
                team_id.as_str(), *base_value_kpo as i32, *cost_kpo as i32, version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::StaffBought { cost_kpo, .. } => {
            sqlx::query!(
                "UPDATE teams_projection
                 SET team_value_kpo = team_value_kpo + $2,
                     treasury_kpo   = GREATEST(treasury_kpo - $2, 0),
                     version        = $3
                 WHERE team_id = $1",
                team_id.as_str(), *cost_kpo as i32, version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::PlayerImprovementApplied { value_delta, .. } => {
            sqlx::query!(
                "UPDATE teams_projection SET team_value_kpo = team_value_kpo + $2, version = $3 WHERE team_id = $1",
                team_id.as_str(), *value_delta as i32, version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::PlayerFired { value_kpo_at_firing, .. }
        | TeamDomainEvent::PlayerNotReEngaged { value_kpo_at_release: value_kpo_at_firing, .. } => {
            sqlx::query!(
                "UPDATE teams_projection SET team_value_kpo = GREATEST(team_value_kpo - $2, 0), version = $3 WHERE team_id = $1",
                team_id.as_str(), *value_kpo_at_firing as i32, version as i64
            ).execute(tx).await?;
        }
        TeamDomainEvent::PlayerValueUpdated { delta_kpo, .. } => {
            sqlx::query!(
                "UPDATE teams_projection
                 SET team_value_kpo = GREATEST(team_value_kpo + $2, 0),
                     version        = $3
                 WHERE team_id = $1",
                team_id.as_str(), *delta_kpo, version as i64
            ).execute(tx).await?;
        }
        // Événements sans impact sur la projection : mise à jour de version uniquement
        _ => {
            sqlx::query!(
                "UPDATE teams_projection SET version = $2 WHERE team_id = $1",
                team_id.as_str(), version as i64
            ).execute(tx).await?;
        }
    }
    Ok(())
}
```

### Requêtes de liste

```rust
// Ajout au trait ITeamRepository (carte 29)
async fn find_by_coach_and_space(
    &self, coach_id: &UserId, space_id: &SpaceId
) -> Result<Vec<TeamSummary>, RepositoryError>;

async fn find_enrolled_by_space(
    &self, space_id: &SpaceId
) -> Result<Vec<TeamSummary>, RepositoryError>;

pub struct TeamSummary {
    pub team_id:              TeamId,
    pub name:                 String,
    pub roster_name:          String,
    pub participation_status: ParticipationStatus,
    pub game_phase:           Option<GamePhase>,
    pub dedicated_fans:       u8,
    pub treasury_kpo:         u32,
}
```

### Reconstruction de la projection (opération de maintenance)

En cas de désynchronisation, la projection peut être reconstruite :
1. `TRUNCATE teams_projection`
2. Pour chaque équipe : `SELECT payload FROM team_event_store ORDER BY version` → rejouer `update_projection_in_tx()` sur chaque événement

Ce n'est pas un use case applicatif — c'est une opération d'administration à déclencher manuellement si besoin.

---

## Checklist

- [ ] Migration `teams_projection` (colonnes + index)
- [ ] `TeamRepository::update_projection_in_tx()` — couvre tous les variants de `TeamDomainEvent`
- [ ] `TeamSummary` struct avec `roster_name`, `dedicated_fans`, `treasury_kpo`
- [ ] Ajouter `find_by_coach_and_space()` + `find_enrolled_by_space()` au trait `ITeamRepository`
- [ ] Implémenter les deux requêtes de liste sur `teams_projection`
- [ ] Test d'intégration : séquence d'appends → projection reflète le bon état à chaque étape
- [ ] Test d'intégration : `find_by_coach_and_space` retourne les bonnes équipes
- [ ] Test d'intégration : rollback transaction → event store ET projection inchangés
