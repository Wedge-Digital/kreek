# Onglets Résultats & Calendrier — Phase 3 : Architecture back

## BC : competitions

---

## Fichiers à créer

### Controllers (handlers)

| Fichier | Rôle |
|---|---|
| `io/web/resultats_tab_controller.rs` | GET initial + GET paginé (cursor) pour l'onglet Résultats |
| `io/web/calendrier_tab_controller.rs` | GET initial + GET paginé (cursor) pour l'onglet Calendrier |

### Templates

| Fichier | Rôle |
|---|---|
| `templates/competition-tab-resultats.html` | Journées Résultats + sentinel (initial : enveloppé dans `#resultats-list`) |
| `templates/competition-tab-calendrier.html` | Journées Calendrier + sentinel (initial : enveloppé dans `#calendrier-list`) |

---

## Fichiers à modifier

### `io/web/competition_detail.rs`

- Supprimer `get_tab_matches` et `MatchesTabTemplate`
- Ajouter les appels aux handlers des deux nouveaux onglets (full-page fallback)
- Remplacer l'onglet "Matchs" par "Résultats" et "Calendrier" dans la template de page

### `templates/competition-detail.html`

- Remplacer l'onglet "Matchs" par deux onglets "Résultats" et "Calendrier"
- Ajouter les conteneurs `#resultats-list` et `#calendrier-list` avec leur `hx-get` lazy

### `templates/competition-tab-matches.html`

- À supprimer (remplacé par les deux nouvelles templates)

### `domain/domain_event.rs` — `PairingCreated`

Enrichir l'event avec les données d'affichage au moment de la création du pairing :

```rust
PairingCreated {
    // champs existants
    event_id: EventId,
    pairing_id: String,
    competition_id: String,
    season_id: String,
    round_id: String,
    home_team_id: String,
    away_team_id: String,
    space_id: String,
    // nouveaux champs display
    home_team_name: String,
    home_roster_name: String,
    home_coach_name: String,
    home_logo_url: Option<String>,
    away_team_name: String,
    away_roster_name: String,
    away_coach_name: String,
    away_logo_url: Option<String>,
    round_date_start: Option<String>,
    round_date_end: Option<String>,
    round_day_type: String,
}
```

---

## Routes (2 nouvelles dans `router.rs`)

```
GET /spaces/:space_id/competitions/:competition_id/seasons/:season_id/resultats
    → resultats_tab_controller::get_resultats_tab

GET /spaces/:space_id/competitions/:competition_id/seasons/:season_id/calendrier
    → calendrier_tab_controller::get_calendrier_tab
```

Les deux endpoints servent à la fois le chargement initial (premier clic sur l'onglet, sans cursor) et les pages suivantes (scroll sentinel, avec `?cursor=journee_id`). Le handler distingue les deux cas via la présence du paramètre `cursor`.

Exposées via `AppRoutes::competitions`.

---

## Pas de use case

Les deux handlers sont des lectures pures sur la projection. Ils appellent directement le repository et construisent les VMs.

---

## Repository — nouvelles méthodes sur `IMatchDayRepository`

```rust
async fn list_resultats(
    &self,
    season_id: &str,
    cursor: Option<&str>,  // journee_id de la dernière journée chargée
    limit: u32,            // 3
) -> Result<Vec<JourneeResultatsDto>, MatchDayRepositoryError>;

async fn list_calendrier(
    &self,
    season_id: &str,
    cursor: Option<&str>,
    limit: u32,            // 3
) -> Result<Vec<JourneeCalendrierDto>, MatchDayRepositoryError>;
```

---

## Projection — table `competition_match_display_proj`

Table de lecture dénormalisée, alimentée par les events (voir Phase 7).

```sql
competition_match_display_proj (
    pairing_id          TEXT PRIMARY KEY,
    season_id           TEXT NOT NULL,
    round_id            TEXT NOT NULL,       -- journée
    round_position      INTEGER NOT NULL,    -- pour l'ordre
    home_team_id        TEXT NOT NULL,
    home_team_name      TEXT NOT NULL,
    home_roster_name    TEXT NOT NULL,
    home_coach_name     TEXT NOT NULL,
    home_logo_url       TEXT,
    home_initials       TEXT NOT NULL,
    away_team_id        TEXT NOT NULL,
    away_team_name      TEXT NOT NULL,
    away_roster_name    TEXT NOT NULL,
    away_coach_name     TEXT NOT NULL,
    away_logo_url       TEXT,
    away_initials       TEXT NOT NULL,
    round_date_start    TEXT,                -- date fixe ou début de plage
    round_date_end      TEXT,                -- NULL si date fixe
    round_day_type      TEXT NOT NULL,       -- fixed_date | time_frame | rest
    match_status        TEXT NOT NULL DEFAULT 'upcoming',  -- upcoming | in_progress | completed
    home_score          INTEGER,
    away_score          INTEGER,
    home_casualties     INTEGER,
    away_casualties     INTEGER,
    match_report_id     TEXT,               -- NULL si pas de rapport démarré
    match_report_url    TEXT                -- NULL si pas de rapport démarré
)
```

### Sources d'alimentation (détaillées en Phase 7)

| Event | Action sur la projection |
|---|---|
| `PairingCreated` (enrichi) | INSERT ligne avec `match_status = 'upcoming'` et données display |
| `PairingDeleted` | DELETE ligne |
| Match report `TeamsConfirmed` (BC match_report) | UPDATE `match_status = 'in_progress'`, `match_report_id`, `match_report_url` |
| Match report `MatchCompleted` (BC match_report) | UPDATE scores, `match_status = 'completed'` |

---

## Logique de filtrage et tri (dans les requêtes SQL)

**`list_resultats`** :
- `WHERE match_status IN ('in_progress', 'completed')`
- `ORDER BY round_position DESC` (journées récentes en premier)
- Cursor : `round_position < :cursor_position`

**`list_calendrier`** :
- `WHERE match_status = 'upcoming'`
- `ORDER BY round_position ASC` (prochaines journées en premier)
- Cursor : `round_position > :cursor_position`

---

## Règles métier confirmées

- Un pairing sans rapport démarré reste dans Calendrier (`match_status = 'upcoming'`), quelle que soit sa date.
- Un match passe dans Résultats dès qu'un rapport est démarré (`in_progress`) ou terminé (`completed`).
- 3 journées par page de scroll pour les deux onglets.
- L'onglet Classement reste l'onglet par défaut.
