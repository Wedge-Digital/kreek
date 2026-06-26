# BC references — Widget inducement-selector

**Priorité : haute**
**Dépend de :** —
**Contexte :** references — nouveau widget

## Objectif

Créer le widget `inducement-selector` dans le BC References. Ce widget affiche les inducements disponibles en 3 onglets (Communs / Spéciaux / Star Players), gère les quantités, et émet un événement DOM `inducementSelectionChanged` à chaque changement.

## Conception

Cf. `docs/specs/match-report/step2-inducements/02-front.md`, `04-dtos.md`

### Nouveaux fichiers

| Fichier | Rôle |
|---|---|
| `io/web/inducement_selector_controller.rs` | Handler GET — filtre les inducements autorisés, construit les VMs, rend le template |
| `io/web/templates/widgets/inducement-selector.html` | 3 onglets, cartes inducements, cartes star players dépliables, qty controls Alpine |
| `assets/static/css/widgets/inducement-selector.css` | Styles du widget |

### Route

```
GET /references/inducement-selector
    ?allowed_inducement_uids=CSV
    &allowed_star_player_uids=CSV
    &roster_id=...
    &instance_id=...
    [&selected=uid:qty,uid:qty]
```

### VMs

- `InducementSelectorItem { uid, name, description, unit_cost, max_qty, category, initial_qty }`
- `StarPlayerSelectorItem { uid, name, rosters_label, cost, ma, st, ag, pa, av, skills, special_ability_name, special_ability_description, initial_qty }`
- `InducementDisplayCategory { Common, Special }` — COMMON/INFAMOUS_STAFF/WIZARD/BIASED_REFEREE → Common ; SPECIALIZED → Special

### Événement DOM émis

```js
htmx.trigger(document.body, 'inducementSelectionChanged', {
  instanceId: String,
  items: [{ uid, name, qty, unit_cost }],
  total_cost: Number
})
```

Émis à chaque changement de qty (Alpine `x-on:change`).

### Isolation

- Racine du widget : `hx-disinherit="*"`
- JS scoped via Alpine `x-data` avec `init()` / `destroy()`
- CSS embarqué via `<link rel="stylesheet">`

## Checklist

- [ ] `InducementSelectorParams` struct (Deserialize)
- [ ] `InducementSelectorItem` + `StarPlayerSelectorItem` VMs
- [ ] Handler : filtre par `allowed_inducement_uids` + `allowed_star_player_uids` + `roster_id`
- [ ] Handler : pré-sélection depuis param `selected`
- [ ] Template : onglet Communs — liste de cartes avec qty +/-
- [ ] Template : onglet Spéciaux — liste de cartes avec qty +/-
- [ ] Template : onglet Star Players — cartes dépliables avec stats + qty +/-
- [ ] Alpine : emit `inducementSelectionChanged` à chaque changement
- [ ] CSS : `inducement-selector.css`
- [ ] Route enregistrée dans `routes.rs` + `router.rs`
- [ ] `hx-disinherit="*"` sur la racine
