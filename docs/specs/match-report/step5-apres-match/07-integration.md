# Step 5 — Effets de bord

## Persistance

Aucune nouvelle méthode repository. Les méthodes existantes suffisent :

| Méthode | Usage |
|---|---|
| `find_by_id(&match_report_id)` | Chargement de l'agrégat dans GET et POST |
| `append_event(event)` | Persistance de `PostMatchRecorded` dans POST |

L'enrichissement de `FanFactorRecorded` (Option B) est transparent côté SQL — les nouveaux champs
`home_dedicated_fans` / `away_dedicated_fans` sont stockés dans le payload JSON de l'événement.
Les anciens événements désérialisent ces champs à `0` via `#[serde(default)]`.

---

## Événements domaine

| Événement | Émis par | Persisté | Réhydraté vers |
|---|---|---|---|
| `PostMatchRecorded` | `record_post_match_use_case` | ✅ event store | `ReadyToPublish` |

Pas d'app events à ce step — `ReadyToPublish` est un état interne au BC `match_report`.
Aucun autre BC n'est notifié avant la publication (scope futur).

---

## Handlers (`io/web/step5_controller.rs`)

### `get_step5`

```rust
pub async fn get_step5(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

| État agrégat | Comportement |
|---|---|
| `Draft` | Redirect vers `edit_match_report` |
| `Cancelled` | `StatusCode::GONE` |
| `PreMatch` | Rendu template — `home_gain`/`away_gain` = `suggest_gains()`, fan mods = 0 |
| `ReadyToPublish` | Rendu template — pré-rempli avec valeurs existantes |

Appels dans le handler :
1. `repo.find_by_id`
2. `ITeamDataPort::find_team_info` × 2 (logos, initiales)
3. `pm.compute_score()`, `pm.compute_cas()`, `pm.suggest_gains()` (ou valeurs existantes si `ReadyToPublish`)

### `post_step5`

```rust
pub async fn post_step5(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordPostMatchForm>,
) -> impl IntoResponse
```

| Résultat use case | Comportement |
|---|---|
| `Ok(Success)` | Redirect vers page récap (route à définir — provisoirement `step5`) |
| `Err(NotFound)` | `StatusCode::NOT_FOUND` |
| `Err(NotInCompatibleState)` | `StatusCode::CONFLICT` |
| `Err(Internal)` | `StatusCode::INTERNAL_SERVER_ERROR` |

---

## Templates

| Fichier | Struct | Notes |
|---|---|---|
| `templates/step5.html` | `Step5Template` | Page complète, formulaire Alpine pour fan mods |

---

## Routes à ajouter (`routes.rs`)

```rust
pub const MATCH_REPORT_STEP5: &str =
    "/app/{space_id}/match-report/{match_report_id}/step5";

// impl Routes
pub fn step5(&self, space_id: &str, match_report_id: &str) -> String {
    path::MATCH_REPORT_STEP5
        .replace("{space_id}", space_id)
        .replace("{match_report_id}", match_report_id)
}
```

---

## Tests E2E (`tests/e2e/`)

| Scénario | Vérifications |
|---|---|
| Accès step5 depuis step4 | Score et sorties affichés, suggestion pré-remplie dans inputs gains |
| Soumission valide (gains + fan mods + résumé) | Redirect, rapport en `ReadyToPublish` |
| Soumission minimale (sans résumé) | Redirect, titre et corps absents tolérés |
| Re-soumission | Formulaire pré-rempli avec valeurs existantes, modification acceptée |
| Accès depuis état `Draft` | Redirect vers edit match report |
