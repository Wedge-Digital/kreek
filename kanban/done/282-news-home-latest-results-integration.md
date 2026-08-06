# BC `news` — intégration du widget "Derniers résultats" sur l'accueil

**Priorité : haute**
**Dépend de :** `281-competitions-latest-results-widget.md`
**Contexte :** `news/io/web/templates/news-feed.html`

## Objectif

Remplacer le bloc statique fictif de la page d'accueil par un chargement
réel du widget `competitions`, via `AppRoutes` (jamais un import direct des
routes d'un autre BC). Spec complète :
`docs/specs/accueil-derniers-resultats/widget-derniers-resultats/07-integration.md`.

---

## Conception

Remplace `news-feed.html:137-202` (bloc `.matches-panel` codé en dur) :

```html
<div class="home-side">
  <div id="latest-results-widget"
       hx-get="{{ app_routes.competitions.latest_results_widget(space_id) }}"
       hx-trigger="load"
       hx-swap="innerHTML">
  </div>
</div>
```

`NewsFeedTemplate` expose déjà `app_routes: AppRoutes` (`news_feed.rs:108`)
— aucune modification du handler nécessaire.

## Checklist

- [ ] Bloc statique de `news-feed.html` retiré, remplacé par le conteneur `hx-get`
- [ ] Vérification manuelle : la page d'accueil charge le widget réel au chargement (`hx-trigger="load"`)
- [ ] `make check-arch` : aucun import direct de `competitions::routes` dans `news`
