# Calendrier — Phase 3 : Architecture back ✅

## BC responsable

Le BC `competitions` possède les journées et les matchs planifiés. Les infos d'équipe pour les selects viennent du port `ITeamInfoPort` (déjà créé pour les poules). Les résultats des matchs passés viendront d'un futur port `IMatchReportPort`.

## Agrégat MatchDay

Chaque journée est un agrégat avec un ID unique (ULID). Cet ID sera utilisé par le futur BC MatchReport.

```rust
pub struct MatchDay {
    pub id: String,
    pub season_id: String,
    pub name: String,
    pub day_type: MatchDayType,  // FixedDate | TimeFrame | Rest
    pub date_start: Option<String>,
    pub date_end: Option<String>,
    pub position: i32,
    pub fixtures: Vec<Fixture>,
}

pub struct Fixture {
    pub id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

pub enum MatchDayType {
    FixedDate,
    TimeFrame,
    Rest,
}
```

Fichier : `src/app/competitions/domain/match_day.rs`

## Persistance

### Tables

```sql
CREATE TABLE competition_match_days (
    id          TEXT PRIMARY KEY,
    season_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    day_type    TEXT NOT NULL DEFAULT 'time_frame',
    date_start  TEXT,
    date_end    TEXT,
    position    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE competition_match_day_fixtures (
    id              TEXT PRIMARY KEY,
    match_day_id    TEXT NOT NULL REFERENCES competition_match_days(id) ON DELETE CASCADE,
    home_team_id    TEXT NOT NULL,
    away_team_id    TEXT NOT NULL
);

CREATE INDEX idx_match_days_season ON competition_match_days (season_id);
CREATE INDEX idx_fixtures_match_day ON competition_match_day_fixtures (match_day_id);
```

### Repository

Trait `IMatchDayRepository` dans `domain/match_day_repository_port.rs` :

- `find_by_season(season_id) -> Vec<MatchDay>`
- `find_by_id(match_day_id) -> Option<MatchDay>`
- `save_match_day(match_day) -> ()`
- `delete_match_day(match_day_id) -> ()`
- `save_fixture(match_day_id, fixture) -> ()`
- `delete_fixture(fixture_id) -> ()`
- `clear_fixtures(match_day_id) -> ()`
- `clear_all_fixtures(season_id) -> ()`
- `ensure_match_days_from_structure(season_id, scheduled_dates) -> ()`

## Synchronisation avec ScheduleConfig

Au premier accès de l'onglet calendrier, les `scheduled_dates` de `CompetitionStructure` sont synchronisées vers `competition_match_days` via `ensure_match_days_from_structure` (même pattern que `ensure_groups_from_structure`).

## Fragment onglet + widgets

### Routes

```
GET  .../admin/schedule                                    → fragment onglet
GET  .../admin/schedule/rounds                             → widget sidebar
GET  .../admin/schedule/round?round_id={id}                → widget détail journée
```

### Fichiers

```
src/app/competitions/io/web/admin/
├── schedule_tab.rs              ← fragment onglet (assemblage)
├── schedule_widgets.rs          ← sidebar + round detail widgets
├── schedule_actions.rs          ← tous les POST/PUT/DELETE handlers
└── templates/admin/
    ├── schedule.html            ← fragment (actions globales + layout split)
    └── widgets/
        ├── schedule-sidebar.html     ← liste journées + boutons ajouter
        └── schedule-round-detail.html ← header + config date + actions matchs + liste matchs + formulaire ajout
```

## Actions

### Routes

```
POST   .../admin/schedule/generate-all                     → générer toutes les rencontres
POST   .../admin/schedule/clear-all                        → vider toutes les rencontres
POST   .../admin/schedule/rounds                           → ajouter une journée
POST   .../admin/schedule/rounds/rest                      → ajouter un repos
PUT    .../admin/schedule/rounds/{round_id}                → modifier dates/type
DELETE .../admin/schedule/rounds/{round_id}                → supprimer une journée
POST   .../admin/schedule/rounds/{round_id}/generate       → générer les rencontres de cette journée
POST   .../admin/schedule/rounds/{round_id}/clear          → vider les matchs de cette journée
POST   .../admin/schedule/rounds/{round_id}/matches        → ajouter un match
DELETE .../admin/schedule/rounds/{round_id}/matches/{match_id} → supprimer un match
```

Tous retournent `HX-Trigger: scheduleChanged`.

## Use cases

### `generate_fixtures.rs`

Génère les rencontres d'une journée par round-robin intra-poule.

1. Charger les groupes de la saison (via `IGroupRepository`)
2. Pour chaque poule, générer les paires home/away non encore jouées
3. Sauvegarder les fixtures avec des IDs uniques

### `generate_all_fixtures.rs`

Itère sur toutes les journées non-repos et appelle `generate_fixtures` pour chacune.

## Ports

- `ITeamInfoPort` (existant) — pour alimenter les TomSelect avec les équipes enrolled
- `IMatchReportPort` (futur) — pour récupérer les scores des matchs passés. Non implémenté maintenant.

## Middleware d'autorisation

Même guard admin que les autres onglets.
