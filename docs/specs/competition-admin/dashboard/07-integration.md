# Dashboard — Phase 7 : Intégration ✅

## Persistance

### Méthodes à ajouter

**`ICompetitionRepository`** (ou event store) :
- `list_recent_events(competition_id: &CompetitionId, limit: usize) -> Vec<StoredEvent>` — retourne les N derniers événements bruts de la compétition

**`ISeasonRepository`** (projections) :
- `count_enrolled_teams(season_id: &SeasonId) -> usize`
- `count_pending_teams(season_id: &SeasonId) -> usize`
- `count_matches(season_id: &SeasonId) -> (played: usize, total: usize)`
- `count_rounds(season_id: &SeasonId) -> (validated: usize, total: usize)`

Ces méthodes sont des requêtes sur les tables de projection existantes ou à créer.

## Handlers

### `admin_page` — page hôte

- Route : `GET /app/{space_id}/competitions/{competition_id}/{season_id}/admin`
- Extracteurs : `Path(space_id, competition_id, season_id)`, `State(AppState)`, `AuthSession`
- Logique :
  1. Charger la compétition
  2. Guard admin (admin espace OU admin compétition) → 403 si non autorisé
  3. Appeler `dashboard_query::execute` pour le contenu inline
  4. Transformer en VMs
  5. Rendre `DashboardFragmentTemplate` → récupérer le HTML string
  6. Rendre `AdminPageTemplate` avec `active_tab: "dashboard"` et `content: dashboard_html`
- Retour : `Result<impl IntoResponse, AppError>`

### `dashboard` — fragment onglet

- Route : `GET /app/{space_id}/competitions/{competition_id}/{season_id}/admin/dashboard`
- Extracteurs : `Path(space_id, competition_id, season_id)`, `State(AppState)`, `AuthSession`
- Logique :
  1. Guard admin → 403
  2. Appeler `dashboard_query::execute`
  3. Transformer en VMs
  4. Rendre `DashboardFragmentTemplate`
- Retour : `Result<impl IntoResponse, AppError>`

## Templates

### `admin-page.html`

Extends `app-layout.html`. Contenu :
- Banner admin (gradient, icône, nom compétition + saison, badge Admin, lien retour)
- Tabs (6 onglets, `active_tab` pour le style actif, chaque tab fait un `hx-get` vers son fragment avec `hx-target="#admin-content"`)
- `<div id="admin-content">{{ content|safe }}</div>`

### `admin/dashboard.html`

Fragment autonome (pas de `extends`). Sections :
- Alertes contextuelles (boucle sur `alerts`)
- Barre de stats (boucle sur `stats`)
- Deux colonnes : progression (boucle sur `progress`) + actions rapides (liens vers les autres onglets)
- Activité récente (boucle sur `activity`)

## CSS

- `assets/static/css/pages/competition-admin.css` — styles partagés par tous les onglets : banner, tabs, panel, actions bar, boutons
- `assets/static/css/pages/competition-admin-dashboard.css` — styles spécifiques dashboard : stats bar, progress bars, activity feed, alert banners, quick actions

## Événements

Aucun — lecture seule.

## Tests E2E

Fichier : `tests/e2e/test_competition_admin_dashboard.py`

Scénarios :
1. **Accès admin** : se connecter en tant qu'admin de la compétition → naviguer vers `/admin` → la page se charge avec le dashboard (banner, tabs, stats visibles)
2. **Accès refusé** : se connecter en tant que coach non-admin → naviguer vers `/admin` → 403
3. **Stats présentes** : vérifier que les chips de stats sont visibles (sélecteurs `.stat-chip`)
4. **Navigation onglets** : cliquer sur l'onglet "Inscriptions" → le contenu de `#admin-content` change, l'URL est mise à jour
