# Dashboard — Phase 5 : Use cases ✅

## Query service : `dashboard_query.rs`

Fichier : `src/app/competitions/use_cases/admin/dashboard_query.rs`

Le dashboard est en lecture seule — pas de commande. Un query service agrège les données depuis les repositories et retourne une struct de synthèse.

### Signature

```rust
pub async fn execute(
    competition_id: &CompetitionId,
    season_id: &SeasonId,
    competition_repo: &dyn ICompetitionRepository,
    season_repo: &dyn ISeasonRepository,
) -> Result<DashboardSummary, QueryError>
```

### Struct de sortie

```rust
pub struct DashboardSummary {
    pub enrolled_count: usize,
    pub pending_count: usize,
    pub matches_played: usize,
    pub matches_total: usize,
    pub rounds_validated: usize,
    pub rounds_total: usize,
    pub max_participants: Option<u32>,
    pub recent_activity: Vec<ActivityEntry>,
}

pub struct ActivityEntry {
    pub kind: ActivityKind,
    pub description: String,
    pub occurred_at: OffsetDateTime,
}

pub enum ActivityKind {
    Enrollment,
    MatchResult,
    Validation,
    RestDay,
}
```

### Orchestration

1. Charger la compétition depuis `competition_repo` → récupérer `max_participants` (via `CompetitionInvitations`)
2. Charger les données de la saison depuis `season_repo` → `enrolled_count`, `pending_count`, `matches_played`, `matches_total`, `rounds_validated`, `rounds_total`
3. Charger les derniers événements depuis l'event store → `list_recent_events(competition_id, limit: 10)` → transformer en `Vec<ActivityEntry>`
4. Retourner `DashboardSummary`

### Erreurs

```rust
pub enum QueryError {
    CompetitionNotFound,
    SeasonNotFound,
    Repository(String),
}
```

### Transformation VMs (dans le handler, pas dans le query service)

Le handler transforme `DashboardSummary` en VMs :

- `pending_count > 0` → alerte "warn" avec lien vers l'onglet inscriptions
- `rounds_validated < rounds_total` et journée en cours terminée → alerte "info" avec lien vers l'onglet résultats
- Les stats chips sont construits à partir des compteurs
- Les barres de progression calculent le pourcentage
- Les `ActivityEntry` sont formatées en `DashboardActivityVm` (description → text, occurred_at → temps relatif "il y a 12 min")
