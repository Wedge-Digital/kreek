# BC match_report — Event store + repository

**Priorité : haute**
**Dépend de :** 88
**Contexte :** match_report step1, couche infrastructure

## Objectif

Créer la migration SQL pour l'event store et la table de projection, implémenter le trait `IMatchReportRepository` et son implémentation PostgreSQL.

## Conception

Cf. `docs/specs/match-report/step1-selection/07-integration.md`

### Migration SQL

```sql
-- match_report_event_store
CREATE TABLE match_report_event_store (
    id                BIGSERIAL   PRIMARY KEY,
    match_report_id   TEXT        NOT NULL,
    event_type        TEXT        NOT NULL,
    event_version     TEXT        NOT NULL DEFAULT '1.0',
    payload           JSONB       NOT NULL,
    version           BIGINT      NOT NULL,
    occurred_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX match_report_es_version ON match_report_event_store (match_report_id, version);
CREATE INDEX match_report_es_id ON match_report_event_store (match_report_id);

-- match_report_projection
CREATE TABLE match_report_projection (
    match_report_id   TEXT        PRIMARY KEY,
    space_id          TEXT        NOT NULL,
    competition_id    TEXT        NOT NULL,
    season_id         TEXT        NOT NULL,
    round_id          TEXT        NOT NULL,
    home_team_id      TEXT        NOT NULL,
    away_team_id      TEXT        NOT NULL,
    created_by        TEXT        NOT NULL,
    origin            TEXT        NOT NULL,
    phase             TEXT        NOT NULL,
    version           BIGINT      NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX match_report_proj_space ON match_report_projection (space_id);
CREATE INDEX match_report_proj_season ON match_report_projection (season_id);
CREATE INDEX match_report_proj_coach ON match_report_projection (created_by, space_id);
```

### Fichiers

```
src/app/match_report/
├── domain/
│   └── match_report_repository_port.rs   ← trait IMatchReportRepository
├── io/
│   └── repository/
│       ├── mod.rs
│       └── match_report_repository.rs    ← implémentation PgPool
```

### Repository

- `append()` : insert event + `update_projection_in_tx()` dans la même transaction. Concurrence optimiste via contrainte unique sur `(match_report_id, version)`.
- `find_by_id()` : charge tous les events, appelle `rehydrate()`, retourne `Option<MatchReportState>`.

### Projection

`update_projection_in_tx()` gère les 3 events step1 :
- `MatchReportCreated` → INSERT
- `SelectionUpdated` → UPDATE home/away
- `SelectionConfirmed` → UPDATE phase = 'PreMatch'

## Checklist

- [ ] Migration SQL (event store + projection + index)
- [ ] `match_report_repository_port.rs` : trait `IMatchReportRepository`
- [ ] `match_report_repository.rs` : implémentation avec `append()`, `find_by_id()`, `update_projection_in_tx()`
- [ ] Test d'intégration : append Created → find_by_id retourne Draft
- [ ] Test d'intégration : append Created + Confirmed → find_by_id retourne PreMatch
- [ ] Test d'intégration : concurrence optimiste rejetée
- [ ] Test d'intégration : projection reflète le bon état
- [ ] `cargo check` passe
