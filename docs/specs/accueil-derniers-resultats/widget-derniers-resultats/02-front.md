# Widget "Derniers résultats" — Phase 2 : Architecture front

## Page hôte

`news-feed.html` (BC `news`) — assemblage pur, le bloc statique existant
(lignes 137-202) est remplacé par un conteneur `hx-get`. Aucune logique JS
propre à ce widget côté page hôte.

## Widget

| Widget | BC | Endpoint GET | Trigger de chargement | Émet | Mode |
|---|---|---|---|---|---|
| Derniers résultats | competitions | `/app/{space_id}/competitions/widget/latest-results` | `load` | rien | Lecture seule |

Aucun événement DOM émis ni écouté — le widget est autonome, pas d'interaction
avec d'autres widgets de la page d'accueil.

## Front vs back

Tout est back. Pas de filtre, pas de toggle, pas de comportement JS/Alpine.
Le lien vers le rapport (`match_report_url`) est un `<a href>` de navigation
complète — pas un `hx-get`, cohérent avec le comportement de
`competition-tab-resultats.html:11` (`<a class="match-widget-link">`).

## Widgets existants réutilisables

Aucun réutilisable tel quel :
- `competition-tab-resultats.html` est scopé à une seule saison
  (`season_id`) et utilise un style de carte différent (`.match-widget`).
- Le widget d'accueil doit couvrir toutes les compétitions/saisons de
  l'espace, avec le style compact déjà maquetté (`.matches-panel` /
  `.match-result`, `assets/static/css/pages/app-home.css:100-121`).

## Intégration dans la page hôte

Remplace `news-feed.html:137-202` :

```html
<div class="home-side">
  <div id="latest-results-widget"
       hx-get="{{ app_routes.competitions.latest_results_widget(space_id) }}"
       hx-trigger="load"
       hx-swap="innerHTML">
  </div>
</div>
```

Accès via `AppRoutes` — le BC `news` n'importe jamais `competitions::routes`
directement (règle CLAUDE.md "Accès aux routes"). `NewsFeedTemplate` expose
déjà `app_routes: AppRoutes` (`news_feed.rs:108`).

## Fragment retourné par le widget

Racine avec `hx-disinherit="*"` (règle obligatoire — la page d'accueil a
d'autres `hx-get` de pagination pouvant injecter des attributs hérités) :

```html
<div class="matches-panel" hx-disinherit="*">
  <div class="matches-panel-title">Derniers résultats</div>

  {% if results.is_empty() %}
  <div class="text-tiny text-dark-3" style="padding: var(--p2) 0;">Aucun résultat pour le moment.</div>
  {% else %}
  {% for r in results %}
  <!-- <a class="match-result" href="..."> si r.report_url.is_some(), sinon <div class="match-result"> -->
  {% endfor %}
  {% endif %}
</div>
```

0 à 4 lignes `.match-result`, selon les règles métier validées (statut
completed, 4 max, tri par `published_at` décroissant, highlight `winner`
neutre en cas d'égalité, lien conditionné à l'autorisation).

## États

| État | Rendu |
|---|---|
| Chargé (1 à 4 résultats) | Bloc `.matches-panel` avec les lignes `.match-result` |
| Vide (aucun match completed) | Bloc `.matches-panel` avec le message "Aucun résultat pour le moment." |
| Erreur serveur | Même rendu que l'état vide — dégradation silencieuse, log serveur uniquement |
