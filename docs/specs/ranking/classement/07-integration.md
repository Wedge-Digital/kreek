# Classement — Phase 7 : Effets de bord (persistance, événements, réponses)

## Persistance

### Table `ranking_lines` (nouvelle, migration)

```sql
CREATE TABLE ranking_lines (
    id               TEXT PRIMARY KEY,               -- ulid
    sequence         BIGSERIAL NOT NULL,              -- ordre d'enregistrement global, non ambigu (cf. règle #5)
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

`sequence` (plutôt que `recorded_at` seul) sert de critère de "dernière ligne" — deux matchs publiés au même instant (même timestamp à la précision près) ne doivent jamais être ambigus (règle métier #5 : la dernière ligne fait foi).

### `IRankingRepository`

```rust
#[async_trait]
pub trait IRankingRepository: Send + Sync {
    /// Dernière ligne d'une équipe pour une saison — None si l'équipe n'a encore joué aucun match.
    async fn find_latest_line(&self, season_id: &str, team_id: &str) -> Result<Option<RankingLineRow>, RepositoryError>;

    /// Dernière ligne de **chaque** équipe ayant au moins une ligne pour la saison (une par team_id, DISTINCT ON (team_id) ... ORDER BY sequence DESC).
    async fn find_latest_lines_for_season(&self, season_id: &str) -> Result<Vec<RankingLineRow>, RepositoryError>;

    /// Insère plusieurs lignes dans une seule transaction — utilisé pour les 2 lignes d'un même match (règle #8, jamais l'une sans l'autre).
    async fn insert_lines(&self, lines: &[RankingLine]) -> Result<(), RepositoryError>;
}
```

Implémentation Postgres : `ranking/io/repository/ranking_repository.rs`.

## Événements

- **Écouté** : `MatchReportAppEvent::MatchReportPublished` (existant, `shared_kernel::app_events::match_report_app_events`) — aucune modification côté `match_report`
- **Émis** : aucun. Rien n'a besoin de savoir qu'une ligne de classement a été enregistrée pour l'instant (règle #identifiée Phase 5)
- **Câblage** (`ranking::context::init_listeners`) :
  ```rust
  pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool, competition_port: Arc<dyn IRankingCompetitionPort>) {
      match_report_published_listener::init(app_event_bus, pool, competition_port);
  }
  ```
  Appelé depuis `main.rs`, avant la construction d'`AppState` (même schéma que `players::context::init_listeners`).

## Handlers

### `classement_widget` (BC `ranking`, nouveau)

```rust
pub async fn classement_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

- 401 si pas de session (même politique que `resultats_tab_controller`/`calendrier_tab_controller` — accès à tout utilisateur connecté, pas réservé aux admins contrairement à `summary_tab`)
- Charge `RankingRulesInfo`, `EnrolledTeamInfo` (port) et `RankingLineRow` (repository) en parallèle (`tokio::join!`, pattern déjà utilisé dans `summary_tab.rs`)
- Construit `ClassementWidgetVm` (via `builders.rs`) et rend `ClassementWidgetTemplate`

### `get_tab_standings` (BC `competitions`, modifié)

```rust
pub async fn get_tab_standings(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse
```

Simplifié : ne calcule plus rien, rend uniquement le wrapper `hx-get` (Phase 2) — fragment si `hx-request`, sinon enveloppé dans `full_page(...)` (pattern déjà en place pour resultats/calendrier).

## Templates

| Template | BC | Consomme | Nouveau / modifié |
|---|---|---|---|
| `ranking/templates/widgets/classement-widget.html` | ranking | `ClassementWidgetVm` | Nouveau |
| `competitions/templates/competition-tab-standings.html` | competitions | rien (juste `space_id`/`competition_id`/`season_id`/`app_routes` pour construire l'URL du widget) | Modifié (simplifié) |

### CSS du widget

`classement-widget.html` embarque son propre stylesheet (`assets/static/css/widgets/classement-widget.css`), avec ses propres règles pour le tableau/états vides/erreur — **pas** de dépendance à `competition-detail.css` (règle CLAUDE.md « CSS embarqué, pas de dépendance au layout »). Les classes `.standings-table`/`.standings-row`/etc. et `.table-error`/`.table-error-zone` existantes dans `competition-detail.css`/`team-build.css` sont dupliquées (adaptées) dans ce nouveau fichier plutôt que réimportées depuis un autre BC.

## Tests E2E prévus (Playwright)

Nouveau fichier `tests/e2e/test_ranking_classement.py` :

1. Aucune équipe inscrite à la saison → onglet Classement affiche "Aucune équipe dans la compétition."
2. Équipes inscrites, aucun rapport de match publié → "Aucun match n'a encore été joué."
3. Règles de classement non configurées pour la saison → état d'erreur affiché (peu importe le nombre d'équipes/matchs)
4. Un rapport de match publié → les 2 équipes apparaissent avec MJ=1 et les V/N/D/Pts corrects selon le score et les règles configurées
5. Deux rapports de match publiés pour la même équipe → la ligne affichée reflète le cumul des deux (MJ=2), pas seulement le dernier match
6. Le tri est strictement décroissant par points (l'équipe avec le plus de points apparaît en rang 1)
