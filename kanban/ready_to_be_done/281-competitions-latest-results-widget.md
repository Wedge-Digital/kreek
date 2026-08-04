# BC `competitions` — widget "Derniers résultats"

**Priorité : haute**
**Dépend de :** `279-competitions-latest-results-repository.md`, `280-competitions-latest-results-authorization.md`
**Contexte :** `competitions/io/web/widgets/`, `competitions/io/web/templates/widgets/`, `competitions/routes.rs`, `competitions/router.rs`

## Objectif

Handler + template + route du widget, en lecture seule (pas de use case,
même pattern que l'onglet Résultats). Spec complète :
`docs/specs/accueil-derniers-resultats/widget-derniers-resultats/02-front.md`
et `07-integration.md`.

---

## Conception

### Route (`routes.rs` / `router.rs`)

```rust
pub const COMPETITION_LATEST_RESULTS_WIDGET: &str = "/app/{space_id}/competitions/widget/latest-results";
```
```rust
pub fn latest_results_widget(&self, sid: &str) -> String {
    path::COMPETITION_LATEST_RESULTS_WIDGET.replace("{space_id}", sid)
}
```

### Handler (`io/web/widgets/latest_results_widget.rs`)

```
GET /app/{space_id}/competitions/widget/latest-results

Extracteurs : AuthSession, Path(space_id), State
1. user absent → StatusCode::UNAUTHORIZED
2. rows = match_day_repository.list_latest_completed_results(space_id, 4)
   Err → tracing::error!, results: vec![] (même rendu que l'état vide)
3. authz = latest_results_view::compute_authorization(&state, &user, &space_id, &rows)
4. results = rows.into_iter().map(|r| to_latest_result_vm(r, &authz)).collect()
5. LatestResultsWidgetTemplate { results }
```

### Template (`templates/widgets/latest-results-widget.html`)

Racine `hx-disinherit="*"` (règle obligatoire, la page hôte a d'autres
`hx-get` de pagination) :

```html
<div class="matches-panel" hx-disinherit="*">
  <div class="matches-panel-title">Derniers résultats</div>
  {% if results.is_empty() %}
  <div class="text-tiny text-dark-3 matches-panel-empty">Aucun résultat pour le moment.</div>
  {% else %}
  {% for r in results %}
  {% if let Some(url) = r.report_url %}<a href="{{ url }}" class="match-result">{% else %}<div class="match-result">{% endif %}
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

`.matches-panel-empty` : nouvelle classe dans `app-home.css` (padding
vertical seul) pour éviter le `style="..."` inline interdit en template
final.

## Checklist

- [ ] `COMPETITION_LATEST_RESULTS_WIDGET` + fonction `latest_results_widget` dans `routes.rs`, enregistrement dans `router.rs`
- [ ] Handler `get_latest_results_widget`
- [ ] Template `latest-results-widget.html`, `hx-disinherit="*"` sur la racine
- [ ] Classe `.matches-panel-empty` dans `app-home.css` (pas de style inline)
- [ ] `cargo check` compile, widget accessible en isolation (URL directe)
