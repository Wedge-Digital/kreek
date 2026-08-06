# BC `competitions` — lecture des derniers résultats d'un espace

**Priorité : haute**
**Dépend de :** `278-competitions-latest-results-projection.md`
**Contexte :** `competitions/domain/match_day_repository_port.rs`, `competitions/io/repository/`

## Objectif

Lister les derniers matchs `completed` d'un espace, toutes compétitions/
saisons confondues, triés par date réelle de publication. Spec complète :
`docs/specs/accueil-derniers-resultats/widget-derniers-resultats/03-back.md`
et `04-dtos.md`.

Jointure vers `competition_seasons` et `competitions` (même BC, pas une
violation de souveraineté) plutôt qu'une dénormalisation de `space_id` /
nom de compétition dans la projection — évite de toucher les 3 points de
création de pairing existants.

---

## Conception

### DTO (`domain/match_day_repository_port.rs`)

```rust
pub struct LatestResultDto {
    pub pairing_id: String,
    pub season_id: String,
    pub competition_id: String,
    pub competition_name: String,
    pub round_name: String,
    pub home_team_id: String,
    pub home_team_name: String,
    pub home_score: Option<i32>,
    pub away_team_id: String,
    pub away_team_name: String,
    pub away_score: Option<i32>,
    pub match_report_url: Option<String>,
    pub published_at: Option<time::OffsetDateTime>,
}
```

### Port

```rust
async fn list_latest_completed_results(&self, space_id: &str, limit: i64)
    -> Result<Vec<LatestResultDto>, MatchDayRepositoryError>;
```

### SQL (`io/repository/sql/match_days/list_latest_results.sql`)

```sql
SELECT cmdp.pairing_id, cmdp.season_id, c.id AS competition_id, c.name AS competition_name,
       cmdp.round_name, cmdp.home_team_id, cmdp.home_team_name, cmdp.home_score,
       cmdp.away_team_id, cmdp.away_team_name, cmdp.away_score,
       cmdp.match_report_url, cmdp.published_at
FROM competition_match_display_proj cmdp
JOIN competition_seasons cs ON cs.id = cmdp.season_id
JOIN competitions c ON c.id = cs.competition_id
WHERE c.space_id = $1 AND cmdp.match_status = 'completed'
ORDER BY cmdp.published_at DESC NULLS LAST
LIMIT $2
```

## Checklist

- [ ] `LatestResultDto` dans `match_day_repository_port.rs`
- [ ] `list_latest_completed_results` sur `IMatchDayRepository` + implémentation `sqlx::query_as!`
- [ ] `list_latest_results.sql`
- [ ] Test d'intégration repository (vraie PgPool, fixture avec 2 compétitions différentes du même espace) : ordre chronologique correct, `NULLS LAST` pour une ligne sans `published_at`, `limit` respecté
