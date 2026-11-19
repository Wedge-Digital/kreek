# BC `teams` — Event store + port `ITeamRepository`

**Priorité : haute**
**Dépend de :** `28-teams-aggregate.md`
**Contexte :** `teams` — infrastructure

## Objectif

Créer l'event store de l'agrégat `Team` et le port `ITeamRepository` avec ses deux opérations fondamentales : appendre un événement et hydrater l'agrégat par rejeu. La gestion de la projection de lecture est traitée dans la carte `42-teams-projection.md`.

---

## Conception

### Schéma SQL — event store

```sql
-- migrations/YYYYMMDD_team_event_store.sql
CREATE TABLE team_event_store (
    id            BIGSERIAL   PRIMARY KEY,
    team_id       TEXT        NOT NULL,
    event_type    TEXT        NOT NULL,               -- discriminant lisible, utile pour les requêtes opérationnelles
    event_version TEXT        NOT NULL DEFAULT '1.0', -- version du schéma du variant, pour évolution sans migration destructive
    payload       JSONB       NOT NULL,               -- TeamDomainEvent sérialisé avec #[serde(tag = "type")]
    version       BIGINT      NOT NULL,               -- séquence par équipe, base 1
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX team_event_store_version ON team_event_store (team_id, version);
CREATE INDEX        team_event_store_team_id ON team_event_store (team_id);
```

**`version`** — séquence monotone **par équipe**. Sert à l'optimistic locking : l'INSERT de la version N+1 échoue sur la contrainte unique si un concurrent a déjà écrit cette position.

**`event_version`** — version du schéma du variant (`"1.0"`, `"1.1"`, …). Les champs ajoutés en versions ultérieures portent `#[serde(default)]` pour rester compatibles avec les anciens enregistrements sans migration.

**`event_type`** — redondant avec `payload->>'type'` mais permet des requêtes sans parsing JSON et facilite le monitoring et les outils d'administration.

**`payload`** — `TeamDomainEvent` sérialisé en internally tagged :
```json
{ "type": "TeamCreated", "name": "Les Korrigans FC", "roster_id": "01J…", "treasury": 1000 }
{ "type": "TeamEnrolled", "competition_id": "01J…", "season_id": "01J…" }
{ "type": "TeamDismissed" }
```

### Port

```rust
// ports.rs
#[async_trait]
pub trait ITeamRepository: Send + Sync {
    /// Appende un événement dans la transaction fournie.
    /// Retourne la nouvelle version de l'agrégat.
    /// Échoue avec ConcurrentWrite si expected_version ne correspond pas à la version courante.
    async fn append(
        &self,
        team_id:          &TeamId,
        event:            &TeamDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError>;

    /// Hydrate l'agrégat en chargeant et rejouant tous ses événements.
    async fn find_by_id(&self, id: &TeamId) -> Result<Option<Team>, RepositoryError>;
}
```

Les requêtes de liste (`find_by_coach_and_space`, `find_enrolled_by_space`) sont définies dans la carte 42 — elles s'appuient sur la table de projection.

### Méthodes utilitaires sur `TeamDomainEvent`

```rust
impl TeamDomainEvent {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::TeamCreated { .. }                  => "TeamCreated",
            Self::TeamEnrolled { .. }                 => "TeamEnrolled",
            Self::TeamDismissed                       => "TeamDismissed",
            Self::MatchPlayedReceived { .. }          => "MatchPlayedReceived",
            Self::PlayerImprovementApplied { .. }     => "PlayerImprovementApplied",
            Self::PlayerImprovementPhaseValidated     => "PlayerImprovementPhaseValidated",
            Self::PlayerRecruited { .. }              => "PlayerRecruited",
            Self::StaffBought { .. }                  => "StaffBought",
            Self::RecruitmentPhaseValidated           => "RecruitmentPhaseValidated",
            Self::PlayerFired { .. }                  => "PlayerFired",
            Self::DismissalsPhaseValidated            => "DismissalsPhaseValidated",
            Self::PlayerRetiredTemporarily { .. }     => "PlayerRetiredTemporarily",
            Self::RetirementPhaseValidated            => "RetirementPhaseValidated",
            Self::CostlyMistakesApplied { .. }        => "CostlyMistakesApplied",
        }
    }

    pub fn schema_version(&self) -> &'static str {
        "1.0" // à surcharger par variant si le schéma évolue
    }
}
```

### Implémentation — `append()`

L'insert dans `team_event_store` et la mise à jour de la projection (carte 42) se font **dans la même transaction** pour garantir la cohérence.

```rust
async fn append(
    &self,
    team_id:          &TeamId,
    event:            &TeamDomainEvent,
    expected_version: u64,
) -> Result<u64, RepositoryError> {
    let new_version   = expected_version + 1;
    let payload       = serde_json::to_value(event).map_err(RepositoryError::Serialization)?;
    let event_type    = event.type_name();
    let event_version = event.schema_version();

    let mut tx = self.pool.begin().await.map_err(RepositoryError::Database)?;

    sqlx::query!(
        "INSERT INTO team_event_store (team_id, event_type, event_version, payload, version)
         VALUES ($1, $2, $3, $4, $5)",
        team_id.as_str(), event_type, event_version, payload, new_version as i64
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref d) if d.constraint() == Some("team_event_store_version") =>
            RepositoryError::ConcurrentWrite,
        other => RepositoryError::Database(other),
    })?;

    // délégué à l'implémentation de la projection (carte 42)
    self.update_projection_in_tx(&mut tx, team_id, event, new_version).await?;

    tx.commit().await.map_err(RepositoryError::Database)?;
    Ok(new_version)
}
```

### Implémentation — `find_by_id()` (rejeu)

```rust
async fn find_by_id(&self, id: &TeamId) -> Result<Option<Team>, RepositoryError> {
    let rows = sqlx::query!(
        "SELECT payload FROM team_event_store
         WHERE team_id = $1 ORDER BY version ASC",
        id.as_str()
    )
    .fetch_all(&self.pool)
    .await
    .map_err(RepositoryError::Database)?;

    if rows.is_empty() { return Ok(None); }

    let events: Vec<TeamDomainEvent> = rows.iter()
        .map(|r| serde_json::from_value(r.payload.clone()))
        .collect::<Result<_, _>>()
        .map_err(RepositoryError::Deserialization)?;

    Ok(Team::hydrate(&events))
}
```

Le dispatch vers le bon variant est automatique grâce à `#[serde(tag = "type")]` — aucune logique de routing manuel.

---

## Points en suspens

- **Snapshot** : reporté ; point d'entrée naturel = fin de saison (décision product)
- **`ConcurrentWrite`** : les use cases retentent-ils ou remontent-ils l'erreur ? À trancher au cas par cas

---

## Checklist

- [ ] Migration `team_event_store` (colonnes + index)
- [ ] `ITeamRepository` dans `ports.rs` — `append` + `find_by_id` uniquement
- [ ] `RepositoryError` : variants `ConcurrentWrite`, `Serialization`, `Deserialization`, `Database`
- [ ] `TeamDomainEvent::type_name()` + `schema_version()`
- [ ] `TeamRepository::append()` — transaction : INSERT event + `update_projection_in_tx()`
- [ ] `TeamRepository::find_by_id()` — SELECT ORDER BY version + `serde_json::from_value` + `Team::hydrate()`
- [ ] Test d'intégration : append × N événements → `find_by_id` retourne l'agrégat correct
- [ ] Test d'intégration : optimistic locking — deux appends sur même version → `ConcurrentWrite`
- [ ] Injecter dans `TeamsContext::new()`
