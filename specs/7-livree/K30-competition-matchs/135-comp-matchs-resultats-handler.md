# 135 — Handler + template onglet Résultats

## Objectif

Implémenter le controller et le template Askama pour l'onglet Résultats (matchs `in_progress` et `completed`, scroll infini, du plus récent au plus ancien).

## Dépendances

- 134 (méthodes repository disponibles)

## Conception détaillée

### `io/web/resultats_tab_controller.rs` (nouveau fichier)

```rust
#[derive(Deserialize)]
pub struct TabCursorQuery {
    pub cursor: Option<i32>,  // round_position de la dernière journée chargée
}

#[derive(Template)]
#[template(path = "competition-tab-resultats.html")]
pub struct ResultatsTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeResultatsVm>,
    pub next_cursor: Option<i32>,
    pub is_initial: bool,
}

pub async fn get_resultats_tab(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<TabCursorQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse
```

**Logique du handler** :
1. Appeler `match_day_repo.list_resultats(season_id, query.cursor, 3)`
2. Grouper les `PairingDisplayDto` par `round_id` → `Vec<JourneeResultatsVm>`
   - Max 3 journées
   - `next_cursor` = `round_position` de la 3ème journée si exactement 3 journées retournées
3. Si HTMX (`HX-Request` header) → retourner `ResultatsTabTemplate`
4. Si navigation directe → retourner full page via `load_page_base()` avec onglet `resultats` actif

**VMs** (dans le même fichier ou `view_models.rs`) :

```rust
pub enum MatchStatusVm {
    Completed { home_score: u32, away_score: u32, home_cas: u32, away_cas: u32 },
    InProgress { report_url: String },
}

pub struct MatchResultatVm {
    pub home_name: String,
    pub home_roster: String,
    pub home_coach: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_roster: String,
    pub away_coach: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
    pub status: MatchStatusVm,
    pub date: String,
}

pub struct JourneeResultatsVm {
    pub label: String,
    pub matches: Vec<MatchResultatVm>,
}
```

### `templates/competition-tab-resultats.html` (nouveau fichier)

Fragment Askama. Structure :
- `{% if is_initial %}<div id="resultats-list">{% endif %}`
- Boucle sur `journees` → section block par journée
- Chaque ligne : logo/initiales, noms, score ou badge selon `status`
- `{% if let Some(cursor) = next_cursor %}` → sentinel HTMX
- `{% if is_initial %}</div>{% endif %}`

Réutiliser les classes CSS de la maquette : `.match-row`, `.team-logo`, `.score-block`, `.status-badge-in-progress`, `.report-link`, `.scroll-sentinel`.

## Checklist

- [ ] `resultats_tab_controller.rs` créé
- [ ] VMs définis et mapping `PairingDisplayDto` → `JourneeResultatsVm`
- [ ] `next_cursor` calculé correctement (None si < 3 journées)
- [ ] Template `competition-tab-resultats.html` créé
- [ ] Sentinel HTMX présent uniquement si `next_cursor.is_some()`
- [ ] `is_initial` contrôle le wrapper `#resultats-list`
- [ ] Fallback navigation directe (full page) fonctionnel
- [ ] `cargo build` passe
