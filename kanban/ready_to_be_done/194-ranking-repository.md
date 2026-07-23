# BC `ranking` — Repository interne (`IRankingRepository` + migration)

**Priorité : haute**
**Dépend de :** `192-ranking-domaine.md`
**Contexte :** `ranking/ports.rs` (complément) + `ranking/io/repository/` + `migrations/`
**Spec :** `docs/specs/ranking/classement/07-integration.md`

## Objectif

Persistance des lignes de classement — table append-only, jamais de `UPDATE`/`DELETE`. Une équipe peut avoir plusieurs lignes pour une même journée ; seule la plus récente par **ordre d'enregistrement global** (colonne `sequence`, pas le timestamp seul) fait foi pour l'affichage.

## Conception

### Migration

```sql
CREATE TABLE ranking_lines (
    id               TEXT PRIMARY KEY,
    sequence         BIGSERIAL NOT NULL,
    competition_id   TEXT NOT NULL,
    season_id        TEXT NOT NULL,
    round_id         TEXT NOT NULL,
    match_report_id  TEXT NOT NULL,
    team_id          TEXT NOT NULL,
    recorded_at      TIMESTAMPTZ NOT NULL,
    matches_played   INTEGER NOT NULL,
    wins             INTEGER NOT NULL,
    draws            INTEGER NOT NULL,
    losses           INTEGER NOT NULL,
    ranking_points   INTEGER NOT NULL
);

CREATE INDEX idx_ranking_lines_latest ON ranking_lines (season_id, team_id, sequence DESC);
```

### Port (ajout dans `ranking/ports.rs`)

```rust
pub struct RankingLineRow {
    pub team_id:        String,
    pub matches_played: u32,
    pub wins:           u32,
    pub draws:           u32,
    pub losses:          u32,
    pub ranking_points:  u32,
}

#[async_trait]
pub trait IRankingRepository: Send + Sync {
    async fn find_latest_line(&self, season_id: &str, team_id: &str) -> Result<Option<RankingLineRow>, RepositoryError>;
    async fn find_latest_lines_for_season(&self, season_id: &str) -> Result<Vec<RankingLineRow>, RepositoryError>;
    async fn insert_lines(&self, lines: &[RankingLine]) -> Result<(), RepositoryError>;
}
```

`find_latest_lines_for_season` : `SELECT DISTINCT ON (team_id) * FROM ranking_lines WHERE season_id = $1 ORDER BY team_id, sequence DESC`.
`insert_lines` : une seule transaction pour toutes les lignes passées (règle métier #8 — les 2 lignes d'un même match, jamais l'une sans l'autre).

### Implémentation

`src/app/ranking/io/repository/ranking_repository.rs` — `PgRankingRepository`, implémente `IRankingRepository` avec `sqlx::query!`/`query_as!`.

## Checklist

- [ ] Migration `ranking_lines` + index
- [ ] `RankingLineRow` + `IRankingRepository` dans `ranking/ports.rs`
- [ ] `PgRankingRepository` (`find_latest_line`, `find_latest_lines_for_season`, `insert_lines` transactionnel)
- [ ] Tests d'intégration (vraie `PgPool`, pas de mock sqlx) : insertion multi-lignes atomique, `find_latest_line` retourne bien la plus récente par `sequence` (pas par `recorded_at`), `find_latest_lines_for_season` retourne une seule ligne par équipe
- [ ] `cargo check` + `cargo test` passent
- [ ] `make check-arch` propre
