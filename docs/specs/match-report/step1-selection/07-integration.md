# Step 1 — Sélection du match : Intégration

## Persistance

### Event store — migration SQL

```sql
CREATE TABLE match_report_event_store (
    id                BIGSERIAL   PRIMARY KEY,
    match_report_id   TEXT        NOT NULL,
    event_type        TEXT        NOT NULL,
    event_version     TEXT        NOT NULL DEFAULT '1.0',
    payload           JSONB       NOT NULL,
    version           BIGINT      NOT NULL,
    occurred_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX match_report_es_version
    ON match_report_event_store (match_report_id, version);
CREATE INDEX match_report_es_id
    ON match_report_event_store (match_report_id);
```

Même schéma que `team_event_store`. Contrainte unique sur `(match_report_id, version)` pour la concurrence optimiste.

### Table de projection (lecture)

```sql
CREATE TABLE match_report_projection (
    match_report_id   TEXT        PRIMARY KEY,
    space_id          TEXT        NOT NULL,
    competition_id    TEXT        NOT NULL,
    season_id         TEXT        NOT NULL,
    round_id          TEXT        NOT NULL,
    home_team_id      TEXT        NOT NULL,
    away_team_id      TEXT        NOT NULL,
    created_by        TEXT        NOT NULL,
    origin            TEXT        NOT NULL,  -- 'Manual' | 'Pairing'
    phase             TEXT        NOT NULL,  -- 'Draft' | 'PreMatch' | 'InProgress' | 'PostMatch' | 'Completed'
    version           BIGINT      NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX match_report_proj_space    ON match_report_projection (space_id);
CREATE INDEX match_report_proj_season   ON match_report_projection (season_id);
CREATE INDEX match_report_proj_coach    ON match_report_projection (created_by, space_id);
```

La projection sert aux requêtes de liste (ex. : "mes rapports en cours") et à la reprise de saisie (identifier la phase sans rehydrater l'agrégat complet).

### Repository trait

```rust
#[async_trait]
pub trait IMatchReportRepository: Send + Sync {
    async fn append(
        &self,
        match_report_id: &MatchReportId,
        event: &MatchReportDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError>;

    async fn find_by_id(
        &self,
        match_report_id: &MatchReportId,
    ) -> Result<Option<MatchReportState>, RepositoryError>;
}
```

`find_by_id` charge tous les events de l'event store, appelle `rehydrate(events)` et retourne le `MatchReportState` typé.

### Projection mise à jour dans la transaction d'append

Même pattern que `team_repository` : `update_projection_in_tx()` dans la même transaction que l'insert dans l'event store.

```rust
async fn update_projection_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    match_report_id: &str,
    event: &MatchReportDomainEvent,
    version: u64,
) -> Result<(), RepositoryError> {
    match event {
        MatchReportDomainEvent::MatchReportCreated { .. } => {
            // INSERT INTO match_report_projection
        }
        MatchReportDomainEvent::SelectionUpdated { home_team_id, away_team_id, .. } => {
            // UPDATE match_report_projection SET home_team_id, away_team_id
        }
        MatchReportDomainEvent::SelectionConfirmed { .. } => {
            // UPDATE match_report_projection SET phase = 'PreMatch'
        }
    }
}
```

## Événements

### App event émis par le BC competitions

`PairingCreated` — nouvel app event à ajouter. Émis quand un pairing est créé (manuellement ou par génération automatique) dans le calendrier.

Fichier : `src/app/shared_kernel/app_events/competitions_app_events.rs`

```rust
PairingCreated {
    event_id: String,
    pairing_id: String,
    season_id: String,
    round_id: String,
    home_team_id: String,
    away_team_id: String,
    space_id: String,
}
```

Le BC competitions doit émettre cet event depuis les use cases `generate_pairings` et `add_match` (schedule admin).

### App event émis par le BC match_report

`MatchReportConfirmed` — émis quand le `SelectionConfirmed` est persisté. Permet au BC teams de passer les deux équipes en `MatchReporting`.

Fichier : `src/app/shared_kernel/app_events/match_report_app_events.rs`

```rust
pub enum MatchReportAppEvent {
    MatchReportConfirmed {
        event_id: String,
        match_report_id: String,
        home_team_id: String,
        away_team_id: String,
        space_id: String,
    },
}
```

### Listener dans le BC teams

Fichier : `src/app/teams/io/app_events/match_report_confirmed_listener.rs`

Écoute `MatchReportConfirmed` → pour chaque équipe (home + away), charge l'agrégat Team, appelle une nouvelle méthode domaine `start_match_reporting()` → produit un `MatchReportingStarted` domain event → append. La `GamePhase` passe de `ReadyToPlay` à `MatchReporting`.

Le variant `MatchReporting` est ajouté à l'enum `GamePhase` du BC teams.

### Listener dans le BC match_report

Fichier : `src/app/match_report/io/app_events/pairing_created_listener.rs`

Écoute `PairingCreated` → construit un `CreateMatchReportCommand` avec `origin: Pairing` → appelle `CreateMatchReportUseCase`.

Initialisation dans `context.rs` :

```rust
pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool) {
    pairing_created_listener::init(app_event_bus, pool);
}
```

## Handlers

### `match_selection_controller.rs`

| Handler | Signature | Retour |
|---------|-----------|--------|
| `new_match_report` | `GET /match-report/new` — `AuthSession`, `State<AppState>`, `Path<space_id>` | `Result<MatchSelectionTemplate, AppError>` |
| `edit_match_report` | `GET /match-report/{id}` — `AuthSession`, `State<AppState>`, `Path<(space_id, match_report_id)>` | `Result<impl IntoResponse, AppError>` — redirige vers la bonne étape si phase != Draft |
| `seasons_fragment` | `GET /match-report/new/seasons` — `Query<competition_id>` | `Result<SeasonOptionsFragment, AppError>` |
| `rounds_fragment` | `GET /match-report/new/rounds` — `Query<season_id>` | `Result<RoundOptionsFragment, AppError>` |
| `teams_fragment` | `GET /match-report/new/teams` — `Query<season_id>` | `Result<TeamOptionsFragment, AppError>` |
| `create_match_report` | `POST /match-report/new` — `AuthSession`, `Form<CreateMatchReportForm>` | `Result<Redirect, AppError>` — redirect vers `/match-report/{id}` |
| `update_match_selection` | `POST /match-report/{id}` — `AuthSession`, `Form<CreateMatchReportForm>` | `Result<Redirect, AppError>` — redirect vers step2 |

### Logique du handler `edit_match_report`

```rust
let state = repo.find_by_id(&id).await?;
match state {
    Some(MatchReportState::Draft(draft)) => {
        // Render MatchSelectionTemplate pré-rempli
    }
    Some(MatchReportState::PreMatch(_)) => {
        // Redirect vers step2
    }
    Some(MatchReportState::InProgress(_)) => {
        // Redirect vers step3
    }
    // ... etc.
    None => Err(AppError::NotFound),
}
```

Cela permet la **reprise de saisie** : l'utilisateur est redirigé vers la bonne étape automatiquement.

## Templates

### Page complète — `match-selection.html`

Template Askama rendant le formulaire step1 complet. Basé sur la maquette `app-match-report-step1.html`. Contient :
- Stepper (étape 1 active)
- Bannière pré-remplissage (conditionnel si `selected.is_some()`)
- Carte formulaire : selects compétition/saison/journée (TomSelect)
- Carte équipes : selects home/away (TomSelect, searchable sur nom+coach) + cartes preview
- Message d'erreur (conditionnel)
- Boutons actions

Attributs HTMX sur les selects pour la cascade :
```html
<select id="competition-select"
        hx-get="{{ routes.match_report.seasons_fragment(space_id) }}"
        hx-trigger="change"
        hx-target="#season-container"
        hx-swap="innerHTML">
```

### Fragments — `fragments/season-options.html`, `round-options.html`, `team-options.html`

Fragments HTML retournant des `<option>` pour TomSelect. Swappés dans le conteneur du select cible.

## Tests E2E

Fichier : `tests/e2e/test_match_report_selection.py`

| Scénario | Description |
|----------|-------------|
| `test_create_match_report_from_scratch` | Coach arrive sur `/match-report/new`, sélectionne compétition/saison/journée/équipes, clique Commencer → redirect vers step2 |
| `test_prefilled_match_report_from_pairing` | Un pairing existe → le coach arrive sur `/match-report/{id}` → formulaire pré-rempli, bannière verte visible, clique Commencer → redirect step2 |
| `test_same_team_error` | Sélectionne la même équipe home et away → message d'erreur affiché |
| `test_cascade_selects` | Change la compétition → les saisons se rechargent, change la saison → les journées se rechargent |
| `test_resume_returns_to_correct_step` | Un rapport en phase PreMatch → arriver sur `/match-report/{id}` redirige vers step2, pas step1 |
| `test_coach_only_sees_own_teams` | Coach lambda → le select "mon équipe" ne contient que ses propres équipes enrolled |
