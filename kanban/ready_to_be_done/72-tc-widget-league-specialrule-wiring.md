# BC `team_creation` — Câblage événementiel League/Special Rule selectors

**Priorité : haute**
**Dépend de :** `69-tc-widget-roster-picker.md`
**Contexte :** BC `team_creation` — page hôte build-team

## Objectif

Remplacer le chargement JS impératif des widgets league-selector et special-rule-selector (IIFE fetch + loadZone) par un câblage déclaratif HTMX, piloté par l'événement `rosterSelected`. Supprimer les fonctions Alpine `leagueSelector()` et `specialRuleSelector()` de la page hôte.

---

## Situation actuelle

- Les sélecteurs de ligue et règle spéciale sont déjà des widgets du BC `references` (endpoints existants)
- Leur chargement est piloté par du JS impératif dans `build-team.html` :
  - `reloadLeagueSelector(rosterUid, selected)` — `fetch()` + `Alpine.initTree()` (~10 lignes)
  - `reloadSpecialRuleSelector(rosterUid, selected)` — idem
  - Event listeners `leagueSelected` et `specialRuleSelected` qui rechargent le sélecteur avec la nouvelle sélection
- Les fonctions Alpine `leagueSelector(onSelectUrl)` et `specialRuleSelector(onSelectUrl)` (~40 lignes) gèrent le callback de sélection via `fetch()` + parsing du header `HX-Trigger`

---

## Conception

### Câblage déclaratif dans la page hôte

```html
<div class="roster-league-col">
  <div id="league-selector-zone"
       hx-get="{{ ref_routes.league_selector_base() }}"
       hx-trigger="rosterSelected from:body"
       hx-vals='js:{
         "roster_id": event.detail.uid,
         "selected": "",
         "on_select": "{{ team_routes.set_league(space_id, team_id) }}"
       }'
       hx-target="this"
       hx-swap="innerHTML">
  </div>
</div>

<div class="roster-league-col">
  <div id="special-rule-selector-zone"
       hx-get="{{ ref_routes.special_rule_selector_base() }}"
       hx-trigger="rosterSelected from:body"
       hx-vals='js:{
         "roster_id": event.detail.uid,
         "selected": "",
         "on_select": "{{ team_routes.set_special_rule(space_id, team_id) }}"
       }'
       hx-target="this"
       hx-swap="innerHTML">
  </div>
</div>
```

### Rechargement après sélection

Les handlers `set_league` et `set_special_rule` retournent déjà un `HX-Trigger` avec `leagueSelected` / `specialRuleSelected`. Pour que les sélecteurs se rechargent après une sélection, deux options :

**Option A — les sélecteurs écoutent aussi leur propre événement :**
```html
hx-trigger="rosterSelected from:body, leagueSelected from:body"
```
Le sélecteur se recharge avec `selected` mis à jour. Mais il faut passer le `selected` depuis l'événement.

**Option B — les handlers set_league / set_special_rule retournent directement le fragment mis à jour :**
Le handler fait un appel interne au endpoint du BC `references` et retourne le fragment. Mais ça violerait la souveraineté.

**Option C (recommandée) — le handler retourne l'événement, le sélecteur se recharge via HTMX :**
```html
<div id="league-selector-zone"
     hx-get="{{ ref_routes.league_selector_base() }}"
     hx-trigger="rosterSelected from:body, leagueSelected from:body"
     hx-vals='js:{
       "roster_id": document.body.dataset.currentRosterId || "",
       "selected": (event.detail?.league_id || ""),
       "on_select": "{{ team_routes.set_league(space_id, team_id) }}"
     }'>
</div>
```

Le `roster_id` courant doit être accessible. Deux solutions :
1. `data-current-roster-id` sur un élément parent, mis à jour par un listener `rosterSelected`
2. Le payload de `leagueSelected` inclut déjà le `roster_id`

**À décider à l'implémentation** : la solution exacte pour passer le `roster_id` au rechargement post-sélection. Le principe reste : tout est déclaratif HTMX, pas de JS impératif.

### JS supprimé de `build-team.html`

Tout le contenu du `<script>` actuel est supprimé :
- `leagueSelector()` — fonction Alpine (~20 lignes)
- `specialRuleSelector()` — fonction Alpine (~20 lignes)
- IIFE complète (~90 lignes) : `loadZone`, `reloadLeagueSelector`, `reloadSpecialRuleSelector`, event listeners, TomSelect init

### Pré-remplissage au chargement

Si un roster est déjà sélectionné (`selected_roster_uid`), les sélecteurs doivent se charger au `load`. Deux approches :
1. La page hôte rend un `<script>` minimal qui émet `rosterSelected` au load si un roster est sélectionné
2. Les sélecteurs ont un `hx-trigger="load"` conditionnel avec les params baked

La solution 2 est préférable (pas de JS) : si `selected_roster_uid` existe, les zones sont rendues avec `hx-trigger="load, rosterSelected from:body"` et les `hx-vals` pré-remplis. Sinon, seulement `hx-trigger="rosterSelected from:body"`.

---

## Situation finale

- **Zéro JS** dans `build-team.html` (le `<script>` entier est supprimé)
- Les sélecteurs league/special-rule sont chargés par HTMX déclaratif
- La coordination roster → league/special-rule passe par `rosterSelected from:body`
- Le rechargement post-sélection passe par `leagueSelected from:body` / `specialRuleSelected from:body`
- Les fonctions Alpine `leagueSelector()` et `specialRuleSelector()` sont supprimées (le callback `on_select` est géré par HTMX nativement dans les widgets du BC `references`)

---

## Point d'attention

Les widgets league-selector et special-rule-selector du BC `references` utilisent actuellement `on_select` comme URL de callback POST. Les fonctions Alpine `leagueSelector()` / `specialRuleSelector()` font un `fetch(onSelectUrl, { method: 'POST' })` puis parsent le header `HX-Trigger`. 

Si ces widgets passent à un `hx-post="{{ on_select }}"` natif HTMX (ce qui est déjà le cas dans la carte 54), le header `HX-Trigger` sera automatiquement traité par HTMX et les événements DOM seront émis sans JS custom. C'est le comportement voulu.

---

## Checklist

- [ ] Remplacer `<div id="league-selector-zone">` par un conteneur HTMX déclaratif avec `hx-trigger="rosterSelected from:body"`
- [ ] Idem pour `<div id="special-rule-selector-zone">`
- [ ] Gérer le pré-remplissage si `selected_roster_uid` existe (hx-trigger load conditionnel)
- [ ] Supprimer `leagueSelector()` de `build-team.html`
- [ ] Supprimer `specialRuleSelector()` de `build-team.html`
- [ ] Supprimer l'IIFE complète (loadZone, reloadLeagueSelector, reloadSpecialRuleSelector, event listeners, TomSelect)
- [ ] Vérifier que les widgets references émettent bien les événements DOM via HTMX natif (pas de JS custom nécessaire côté page hôte)
- [ ] Supprimer `league_selector_url` et `special_rule_selector_url` de `BuildTeamTemplate` (plus nécessaire)
- [ ] Supprimer les imports `ref_routes` de `BuildTeamTemplate` si plus utilisés
- [ ] Test E2E : sélection roster → league selector chargé → sélection ligue → sélecteur rechargé avec sélection
