# BC `players` — Projection `players_projection`

**Priorité : haute**
**Dépend de :** `62-players-bc-event-store-repository.md`
**Contexte :** BC `players` — read model

## Objectif

Créer la table de projection lisible `players_projection`, mise à jour dans la même
transaction que l'append de `PlayerCreated`, et exposer un port de lecture dédié.

---

## Règle fondamentale (rappel CLAUDE.md)

La mise à jour de `players_projection` doit s'exécuter **dans la même transaction**
que l'append de l'event dans `players_events`. Si la transaction échoue, ni l'event
ni la projection ne sont écrits.

---

## Migration

```sql
-- migrations/XXXX_create_players_projection.sql
CREATE TABLE players_projection (
    player_id        TEXT    PRIMARY KEY,
    team_id          TEXT    NOT NULL,
    space_id         TEXT    NOT NULL,
    position_name    TEXT    NOT NULL,
    roster_line_id   TEXT    NOT NULL,
    personal_name    TEXT    NOT NULL DEFAULT '',
    jersey           SMALLINT,
    base_skills      JSONB   NOT NULL DEFAULT '[]',
    acquired_skills  JSONB   NOT NULL DEFAULT '[]',
    spp              INT     NOT NULL DEFAULT 0,
    value_kpo        INT     NOT NULL DEFAULT 0,
    version          INT     NOT NULL DEFAULT 1
);

CREATE INDEX idx_players_projection_team_id ON players_projection (team_id);
```

---

## Port de lecture

```rust
// src/app/players/ports.rs (ajout)
#[async_trait]
pub trait IPlayerProjectionRepository: Send + Sync {
    async fn find_by_team_id(
        &self,
        team_id: &TeamId,
    ) -> Result<Vec<PlayerProjection>, RepositoryError>;
}

pub struct PlayerProjection {
    pub player_id:       String,
    pub team_id:         String,
    pub position_name:   String,
    pub roster_line_id:  String,
    pub personal_name:   String,
    pub jersey:          Option<i16>,
    pub base_skills:     Vec<String>,
    pub acquired_skills: Vec<AcquiredSkillProjection>,
    pub spp:             i32,
    pub value_kpo:       i32,
}
```

---

## Mise à jour transactionnelle

Le use case de création (carte 64) reçoit un `&mut PgConnection` (transaction) et
appelle deux fonctions :

```rust
pub async fn insert_player_event(
    tx:     &mut PgConnection,
    event:  &PlayerDomainEvent,
    version: i32,
) -> Result<(), RepositoryError>;

pub async fn upsert_player_projection(
    tx:     &mut PgConnection,
    event:  &PlayerDomainEvent,
) -> Result<(), RepositoryError>;
```

Les deux roulent dans la même `tx` — commit atomique.

---

## Checklist

- [ ] Migration `players_projection` avec index
- [ ] `PlayerProjection` struct (view model)
- [ ] `IPlayerProjectionRepository::find_by_team_id()`
- [ ] `PgPlayerProjectionRepository` implémentée
- [ ] `insert_player_event()` + `upsert_player_projection()` prenant `&mut PgConnection`
- [ ] `PlayersContext` enrichi avec `projection_repository`
