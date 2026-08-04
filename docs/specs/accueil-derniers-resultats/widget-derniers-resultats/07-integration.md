# Widget "Derniers résultats" — Phase 7 : Intégration

---

## Migration SQL

```sql
ALTER TABLE competition_match_display_proj ADD COLUMN published_at TIMESTAMPTZ;
```

Pas de backfill : les lignes `completed` déjà en base restent à `NULL` et
sortent en fin de tri (`NULLS LAST`, règle métier n°7 validée en Phase 5).
Pas de nouvel index dédié — le volume attendu par espace (nombre de
compétitions × matchs) reste faible ; à revisiter si `list_latest_results.sql`
devient mesurablement lent en usage réel.

---

## Événements — un seul point de code modifié, pas de nouveau listener

### BC competitions — `match_report_published_listener::update_projection`

Pas de nouvel event, pas de nouveau listener : l'UPDATE existant (déclenché
par l'app event `MatchReportPublished`, déjà géré) ajoute une colonne :

```sql
UPDATE competition_match_display_proj
SET match_status = 'completed',
    home_score = $2,
    away_score = $3,
    home_casualties = $4,
    away_casualties = $5,
    match_report_url = $6,
    published_at = $7
WHERE pairing_id = $1
```

`$7` = conversion `chrono::DateTime<Utc>` → `time::OffsetDateTime` de
`payload.published_at` (cf. `04-dtos.md`, pattern `ranking_repository.rs:126-132`).

---

## Requête SQL — `list_latest_results.sql`

```sql
SELECT cmdp.pairing_id, cmdp.season_id, c.id AS competition_id, c.name AS competition_name,
       cmdp.round_name, cmdp.home_team_id, cmdp.home_team_name, cmdp.home_score,
       cmdp.away_team_id, cmdp.away_team_name, cmdp.away_score,
       cmdp.match_report_url, cmdp.published_at
FROM competition_match_display_proj cmdp
JOIN competition_seasons cs ON cs.id = cmdp.season_id
JOIN competitions c ON c.id = cs.competition_id
WHERE c.space_id = $1 AND cmdp.match_status = 'completed'
ORDER BY cmdp.published_at DESC NULLS LAST
LIMIT $2
```

Appelée avec `limit = 4` (règle métier n°2).

---

## Handler

### `widgets/latest_results_widget::get_latest_results_widget`

```
GET /app/{space_id}/competitions/widget/latest-results

Extracteurs : AuthSession, Path(space_id), State
Comportement :
  1. auth_session.user absent → StatusCode::UNAUTHORIZED (cohérent avec le
     reste de /app, déjà gardé par login_required en amont — filet de sécurité)
  2. rows = match_day_repository.list_latest_completed_results(space_id, 4)
     Err → tracing::error!, fragment identique à l'état vide (règle métier n°6)
  3. authz = latest_results_view::compute_authorization(&state, &user, space_id, &rows)
  4. results = rows.into_iter().map(|r| to_latest_result_vm(r, &authz)).collect()
  5. LatestResultsWidgetTemplate { results }
```

---

## Templates

### `templates/widgets/latest-results-widget.html`

```html
<div class="matches-panel" hx-disinherit="*">
  <div class="matches-panel-title">Derniers résultats</div>

  {% if results.is_empty() %}
  <div class="text-tiny text-dark-3" style="padding: var(--p2) 0;">Aucun résultat pour le moment.</div>
  {% else %}
  {% for r in results %}
  {% if let Some(url) = r.report_url %}
  <a href="{{ url }}" class="match-result">
  {% else %}
  <div class="match-result">
  {% endif %}
    <div class="match-league">{{ r.competition_name }} · {{ r.round_name }}</div>
    <div class="match-teams">
      <div class="match-team-row">
        <div class="match-team-name">{{ r.home_name }}</div>
        <div class="match-score {% if r.home_is_winner %}winner{% endif %}">{{ r.home_score }}</div>
      </div>
      <div class="match-team-row">
        <div class="match-team-name">{{ r.away_name }}</div>
        <div class="match-score {% if r.away_is_winner %}winner{% endif %}">{{ r.away_score }}</div>
      </div>
    </div>
    <div class="match-date">{{ r.date }}</div>
  {% if r.report_url.is_some() %}</a>{% else %}</div>{% endif %}
  {% endfor %}
  {% endif %}
</div>
```

Note : le `style="padding..."` de l'état vide reprend celui déjà validé en
Phase 1 sur la maquette (`assets/rawpages/html/app-home.html`) — les
maquettes sont exemptées de l'interdiction des styles inline (CLAUDE.md),
**pas** le template final. À reporter en Phase 8 comme point d'implémentation :
soit une classe utilitaire existante couvre déjà ce padding, soit une petite
classe dédiée est ajoutée à `app-home.css`.

### `news-feed.html` (modification)

Remplace les lignes 137-202 (bloc statique) :

```html
<div class="home-side">
  <div id="latest-results-widget"
       hx-get="{{ app_routes.competitions.latest_results_widget(space_id) }}"
       hx-trigger="load"
       hx-swap="innerHTML">
  </div>
</div>
```

---

## Tests E2E (Playwright)

| Scénario | Vérification |
|---|---|
| Espace avec ≥ 4 matchs `completed` | Widget affiche exactement 4 résultats, du plus récent au plus ancien |
| Espace avec 1-3 matchs `completed` | Widget affiche le nombre exact de résultats disponibles |
| Espace sans match `completed` | Message "Aucun résultat pour le moment." |
| Match nul (scores égaux) | Aucun des deux scores n'a la classe `winner` |
| Coach d'une des deux équipes du match | Résultat cliquable, navigue vers le rapport |
| Coach d'aucune équipe, non admin | Résultat affiché mais non cliquable (pas de `<a>`) |
| Admin d'espace | Tous les résultats cliquables, quelle que soit la compétition |
| Résultats de compétitions différentes | `competition_name` correct affiché pour chaque ligne |
