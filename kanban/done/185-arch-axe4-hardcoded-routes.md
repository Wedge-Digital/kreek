# Architecture — Axe 4 : routes en dur dans les templates `competitions`

**Priorité : haute**
**Dépend de :** rien
**Contexte :** `competitions` — templates HTMX

## Objectif

Deux templates construisent une URL HTMX en dur au lieu de passer par `routes.*`, violation directe de la règle "Accès aux routes" du CLAUDE.md.

```html
<!-- competition-tab-calendrier.html:51 -->
hx-get="/app/{{ space_id }}/competitions/{{ competition_id }}/seasons/{{ season_id }}/calendrier?cursor={{ cursor }}"

<!-- competition-tab-resultats.html:72 -->
hx-get="/app/{{ space_id }}/competitions/{{ competition_id }}/seasons/{{ season_id }}/resultats?cursor={{ cursor }}"
```

## Action

1. Identifier la méthode `routes.*` déjà utilisée pour le premier chargement de chaque onglet (calendrier / résultats) dans le handler correspondant.
2. Vérifier si cette méthode accepte déjà un paramètre `cursor` optionnel ; sinon l'ajouter.
3. Remplacer les deux `hx-get` en dur par l'appel à `routes.competitions.<...>(&space_id, &competition_id, &season_id, cursor)` (nom exact à adapter à l'API réelle de `AppRoutes`).
4. Vérifier que le fragment retourné par le handler correspondant re-render bien le même template avec la nouvelle valeur de `cursor` (pagination).

## Checklist

- [ ] `competition-tab-calendrier.html` utilise `routes.*`
- [ ] `competition-tab-resultats.html` utilise `routes.*`
- [ ] Pagination (`cursor`) toujours fonctionnelle après le changement (vérif manuelle ou test e2e existant)
- [ ] `make check-arch` : axe 4 passe
