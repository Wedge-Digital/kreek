# BC `players` — Event Store & Repository

**Priorité : haute**
**Dépend de :** `61-players-bc-structure-aggregate.md`
**Contexte :** BC `players` — couche infrastructure

## Objectif

Implémenter la persistence event-sourcée du BC `players` : table `players_events`,
port `IPlayerRepository`, et implémentation Postgres.

---

## Migration

```sql
-- migrations/XXXX_create_players_events.sql
CREATE TABLE players_events (
    id           BIGSERIAL PRIMARY KEY,
    player_id    TEXT        NOT NULL,
    team_id      TEXT        NOT NULL,
    event_type   TEXT        NOT NULL,
    payload      JSONB       NOT NULL,
    version      INT         NOT NULL,
    occurred_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (player_id, version)
);

CREATE INDEX idx_players_events_player_id ON players_events (player_id);
CREATE INDEX idx_players_events_team_id   ON players_events (team_id);
```

Le champ `team_id` est dénormalisé dans la table d'events pour permettre une requête
`find_by_team_id` sans jointure supplémentaire.

---

## Port

```rust
// src/app/players/ports.rs
#[async_trait]
pub trait IPlayerRepository: Send + Sync {
    async fn append(
        &self,
        player_id: &PlayerId,
        team_id:   &TeamId,
        event:     &PlayerDomainEvent,
        version:   i32,
    ) -> Result<(), RepositoryError>;

    async fn find_by_id(
        &self,
        player_id: &PlayerId,
    ) -> Result<Option<Player>, RepositoryError>;

    async fn find_by_team_id(
        &self,
        team_id: &TeamId,
    ) -> Result<Vec<Player>, RepositoryError>;
}
```

`find_by_id` et `find_by_team_id` reconstruisent le Player en rejouant les events
(`SELECT … ORDER BY version ASC` → `Player::apply(events)`).

---

## Implémentation Postgres

```rust
// src/app/players/io/repository/player_repository.rs
pub struct PgPlayerRepository { pool: PgPool }
```

- `append` : `INSERT INTO players_events` avec gestion du conflit de version (`ON CONFLICT → RepositoryError::ConcurrentWrite`)
- `find_by_id` : sélectionne tous les events `WHERE player_id = $1 ORDER BY version`
- `find_by_team_id` : sélectionne tous les events `WHERE team_id = $1 ORDER BY player_id, version`, puis groupe par `player_id` et applique

---

## Context

```rust
// src/app/players/context.rs
pub struct PlayersContext {
    pub repository: Arc<dyn IPlayerRepository>,
}
```

Injecté dans `AppState` depuis `main.rs`.

---

## Checklist

- [ ] Migration `players_events` avec index
- [ ] Port `IPlayerRepository` avec `RepositoryError`
- [ ] `PgPlayerRepository::append()` avec contrainte de version
- [ ] `PgPlayerRepository::find_by_id()` avec rebuild par replay
- [ ] `PgPlayerRepository::find_by_team_id()` avec rebuild groupé
- [ ] `PlayersContext` câblé dans `AppState`
