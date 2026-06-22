# Calendrier — Phase 7 : Intégration ✅

## Persistance

### Migration

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

CREATE TABLE competition_match_day_pairings (
    id              TEXT PRIMARY KEY,
    match_day_id    TEXT NOT NULL REFERENCES competition_match_days(id) ON DELETE CASCADE,
    home_team_id    TEXT NOT NULL,
    away_team_id    TEXT NOT NULL
);

CREATE INDEX idx_match_days_season ON competition_match_days (season_id);
CREATE INDEX idx_pairings_match_day ON competition_match_day_pairings (match_day_id);
```

### Repository

Trait `IMatchDayRepository` dans `domain/match_day_repository_port.rs` :

```rust
#[async_trait]
pub trait IMatchDayRepository: Send + Sync {
    async fn find_by_season(&self, season_id: &str) -> Result<Vec<MatchDay>, RepositoryError>;
    async fn find_by_id(&self, match_day_id: &str) -> Result<Option<MatchDay>, RepositoryError>;
    async fn save_match_day(&self, match_day: &MatchDay) -> Result<(), RepositoryError>;
    async fn delete_match_day(&self, match_day_id: &str) -> Result<(), RepositoryError>;
    async fn save_pairing(&self, match_day_id: &str, pairing: &Pairing) -> Result<(), RepositoryError>;
    async fn delete_pairing(&self, pairing_id: &str) -> Result<(), RepositoryError>;
    async fn clear_pairings(&self, match_day_id: &str) -> Result<(), RepositoryError>;
    async fn clear_all_pairings(&self, season_id: &str) -> Result<(), RepositoryError>;
    async fn ensure_match_days_from_structure(&self, season_id: &str, scheduled_dates: &[ScheduledDate]) -> Result<(), RepositoryError>;
}
```

Implémentation : `src/app/competitions/io/repository/match_day_repository.rs`

Câblé dans `CompetitionsContext` : `pub match_day_repository: Arc<dyn IMatchDayRepository>`

### Sync depuis CompetitionStructure

`ensure_match_days_from_structure` : au premier chargement du widget sidebar, lit les `scheduled_dates` depuis `CompetitionStructure` et crée les entrées correspondantes dans `competition_match_days` (ON CONFLICT DO NOTHING). Même pattern que `ensure_groups_from_structure`.

## Handlers

### `schedule_tab.rs`

- Route : `GET .../admin/schedule`
- HTMX → fragment seul / accès direct → page complète via `render_admin_page`
- Rend `ScheduleTabTemplate` (assemblage)

### `schedule_widgets.rs`

**Sidebar** :
- Route : `GET .../admin/schedule/rounds`
- Charge les match days depuis le repository
- Sync depuis structure au premier appel
- Rend `RoundSidebarTemplate`

**Round detail** :
- Route : `GET .../admin/schedule/round?round_id={id}`
- Charge le match day + ses pairings
- Enrichit les pairings avec noms d'équipe via `ITeamInfoPort`
- Charge la liste des équipes enrolled pour les TomSelect
- Rend `RoundDetailTemplate`

### `schedule_actions.rs`

10 routes, toutes retournent `HX-Trigger: scheduleChanged` :

```
POST   .../admin/schedule/generate-all
POST   .../admin/schedule/clear-all
POST   .../admin/schedule/rounds
POST   .../admin/schedule/rounds/rest
PUT    .../admin/schedule/rounds/{round_id}
DELETE .../admin/schedule/rounds/{round_id}
POST   .../admin/schedule/rounds/{round_id}/generate
POST   .../admin/schedule/rounds/{round_id}/clear
POST   .../admin/schedule/rounds/{round_id}/matches
DELETE .../admin/schedule/rounds/{round_id}/matches/{match_id}
```

## Templates

### `admin/schedule.html`

Fragment assemblage :
- Actions globales (info résumé + boutons vider/générer toutes les rencontres)
- Layout split : sidebar + detail
- Sidebar via `hx-get` + `hx-trigger="load, scheduleChanged from:body"`
- Detail via `hx-get` + `hx-trigger="roundSelected from:body, scheduleChanged from:body"`

### `admin/widgets/schedule-sidebar.html`

- Liste des journées (boucle) avec numéro, nom, date, badge statut/type
- Chaque journée émet `roundSelected` au clic
- Boutons "Ajouter une journée" et "Ajouter un repos" en bas

### `admin/widgets/schedule-round-detail.html`

- Header : nom + badge statut + meta (nb matchs) + boutons modifier/supprimer
- Config date : toggle date fixe/plage + inputs date
- Actions matchs : info compteur + boutons vider/générer/ajouter
- Formulaire ajout match (caché par défaut) : deux TomSelect enrichis (nom + coach · roster) + boutons ajouter/annuler
- Liste des pairings : home vs away + tag poule + bouton supprimer
- Résumé en bas

## CSS

`assets/static/css/pages/competition-admin-schedule.css`

Styles : schedule-layout, round-sidebar, round-detail, round-date-config, match-list, add-match-form, TomSelect custom (ts-team-option, ts-team-name, ts-team-meta).

## Tests E2E

Fichier : `tests/e2e/test_competition_admin_schedule.py`

1. **Onglet se charge** : accéder à l'onglet calendrier → sidebar visible avec les journées
2. **Sélection journée** : cliquer une journée → le détail s'affiche à droite
3. **Ajouter une journée** : cliquer "Ajouter une journée" → la sidebar se met à jour
4. **Générer les rencontres** : cliquer "Générer les rencontres" → les pairings apparaissent dans le détail
