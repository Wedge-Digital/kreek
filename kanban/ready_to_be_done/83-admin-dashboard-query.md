# BC `competitions` — Query service dashboard admin

**Priorité : haute**
**Dépend de :** carte 82 (page hôte admin)
**Contexte :** BC `competitions` — administration de compétition
**Spec :** `docs/specs/competition-admin/dashboard/05-use-cases.md`

## Objectif

Créer le query service qui agrège les données de synthèse du dashboard : compteurs (équipes, matchs, journées), et activité récente extraite de l'event store.

---

## Fichiers à créer

| Fichier | Rôle |
|---|---|
| `src/app/competitions/use_cases/admin/mod.rs` | Module admin use cases |
| `src/app/competitions/use_cases/admin/dashboard_query.rs` | Query service + structs de sortie |

## Fichiers à modifier

| Fichier | Modification |
|---|---|
| `src/app/competitions/use_cases/mod.rs` | Ajouter `pub mod admin;` |
| Repositories (ports ou impls) | Ajouter les méthodes de comptage et de listing d'events |

## Détails

### Structs

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

### Signature

```rust
pub async fn execute(
    competition_id: &CompetitionId,
    season_id: &SeasonId,
    competition_repo: &dyn ICompetitionRepository,
    season_repo: &dyn ISeasonRepository,
) -> Result<DashboardSummary, QueryError>
```

### Orchestration

1. Charger la compétition → récupérer `max_participants`
2. Charger les compteurs de la saison (enrolled, pending, matches, rounds)
3. Charger les derniers événements de l'event store (limit 10)
4. Transformer les événements bruts en `Vec<ActivityEntry>`
5. Retourner `DashboardSummary`

### Méthodes repository à ajouter

- `count_enrolled_teams(season_id) -> usize`
- `count_pending_teams(season_id) -> usize`
- `count_matches(season_id) -> (usize, usize)` (played, total)
- `count_rounds(season_id) -> (usize, usize)` (validated, total)
- `list_recent_events(competition_id, limit) -> Vec<StoredEvent>`

---

## Checklist

- [ ] Créer `src/app/competitions/use_cases/admin/mod.rs`
- [ ] Créer `dashboard_query.rs` avec `DashboardSummary`, `ActivityEntry`, `ActivityKind`
- [ ] Implémenter `execute()` avec l'orchestration
- [ ] Ajouter les méthodes de comptage sur les repositories
- [ ] Ajouter `list_recent_events` sur le repository event store
- [ ] Test unitaire : vérifier que le service retourne les bons compteurs
