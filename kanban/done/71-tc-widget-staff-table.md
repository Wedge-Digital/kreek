# BC `team_creation` — Widget Staff & Rerolls autonome

**Priorité : haute**
**Dépend de :** `66-tc-acl-reference-data-port.md`, `67-tc-vm-rapatriement.md`, `68-tc-widget-cart.md`
**Contexte :** BC `team_creation` — widget HTMX

## Objectif

Transformer le tableau de staff et rerolls en widget autonome. La widget écoute `rosterSelected` pour se charger (le staff dépend du roster choisi), gère ses propres mutations buy/remove, et émet `teamMutated`.

---

## Situation actuelle

- Le staff est rendu via la macro `staff_body()` de `staff-fragment.html` dans `build-team.html` (lignes 111–129)
- Quand un roster est sélectionné, la réponse `get_roster_players` inclut le staff en OOB swap (`staff_table_oob` macro)
- Les handlers `buy_staff`, `remove_staff`, `buy_reroll`, `remove_reroll` retournent des fragments individuels (`staff-row-fragment.html`, `reroll-row-fragment.html`) avec OOB cart
- Les fragments row vivent dans `references/io/web/templates/` (violation)

---

## Conception

### Endpoints

```
GET  /app/{space_id}/team/{team_id}/widgets/staff-table                ← chargement / rechargement
POST /app/{space_id}/team/{team_id}/widgets/staff-table/buy-staff      ← acheter un staff
POST /app/{space_id}/team/{team_id}/widgets/staff-table/remove-staff   ← retirer un staff
POST /app/{space_id}/team/{team_id}/widgets/staff-table/buy-reroll     ← acheter une relance
POST /app/{space_id}/team/{team_id}/widgets/staff-table/remove-reroll  ← retirer une relance
```

Note : les routes actuelles (buy_staff, remove_staff, buy_reroll, remove_reroll) peuvent être migrées vers ces nouvelles URLs ou conservées. À décider à l'implémentation.

### Handler GET (chargement)

```rust
pub async fn staff_table_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Charger RosterSelectedTeam
    // Construire staff_rows + reroll VM
    // Si pas de roster sélectionné → état vide
}
```

### Handlers POST mutation

Mêmes handlers qu'aujourd'hui, mais :
1. Retournent uniquement le `<tr>` mis à jour (pas d'OOB cart)
2. Ajoutent `HX-Trigger: teamMutated` dans la réponse

### Templates

Créer dans `team_creation/io/web/templates/widgets/` :
- `staff-table-widget.html` — le tableau complet (thead + tbody avec staff rows + reroll row)
- `staff-row-fragment.html` — fragment d'une ligne staff individuelle (déplacé depuis `references`)
- `reroll-row-fragment.html` — fragment de la ligne reroll (déplacé depuis `references`)

Modifications :
- Supprimer les `{% import "cart-fragment.html" %}` et les blocs OOB cart de chaque fragment
- Ajouter `hx-disinherit="*"` à l'élément racine du widget
- Le widget porte son propre `<link rel="stylesheet">` si nécessaire

### Nettoyage macros

- Supprimer `staff-fragment.html` (les macros `staff_body` et `staff_table_oob` ne sont plus utilisées)
- Supprimer les anciens `staff-row-fragment.html` et `reroll-row-fragment.html` de `references/io/web/templates/`

### Intégration dans la page hôte `build-team.html`

```html
<div class="section-label">4 — Sélectionnez votre staff</div>
<div id="staff-table-container"
     hx-get="{{ team_routes.staff_table_widget(space_id, team_id) }}"
     hx-trigger="load, rosterSelected from:body"
     hx-target="this"
     hx-swap="innerHTML">
</div>
```

### Communication événementielle

| Événement reçu | Action |
|---|---|
| `rosterSelected from:body` | Recharge la widget (le roster détermine le staff dispo) |

| Événement émis | Quand |
|---|---|
| `teamMutated` (via `HX-Trigger` header) | Après chaque buy/remove réussi |

---

## Situation finale

- Le staff+rerolls est une widget autonome dans `team_creation/io/web/widgets/staff_table_widget.rs`
- Les templates vivent dans `team_creation/io/web/templates/widgets/`
- `staff-fragment.html` (macros) est supprimé
- Les fragments de `references/io/web/templates/` (`staff-row-fragment.html`, `reroll-row-fragment.html`) sont supprimés
- La widget écoute `rosterSelected` et émet `teamMutated`
- Aucun OOB swap dans les réponses de mutation
- La page hôte ne contient plus de HTML statique pour le staff
- Le handler GET `get_roster_players` ne retourne plus le staff en OOB (découplé)

---

## Checklist

- [ ] Ajouter les routes widget dans `team_creation/routes.rs`
- [ ] Créer `team_creation/io/web/widgets/staff_table_widget.rs` avec les handlers GET/buy/remove
- [ ] Créer `widgets/staff-table-widget.html` (tableau complet)
- [ ] Déplacer `staff-row-fragment.html` → `widgets/staff-row-fragment.html` (supprimer OOB cart)
- [ ] Déplacer `reroll-row-fragment.html` → `widgets/reroll-row-fragment.html` (supprimer OOB cart)
- [ ] Modifier les handlers de mutation : retourner uniquement le `<tr>` + `HX-Trigger: teamMutated`
- [ ] Supprimer `staff-fragment.html` (macros `staff_body` et `staff_table_oob`)
- [ ] Supprimer les anciens fragments de `references/io/web/templates/`
- [ ] Modifier `build-team.html` : remplacer le tableau statique par `hx-get` + `hx-trigger="load, rosterSelected from:body"`
- [ ] Supprimer l'OOB staff de `get_roster_players` / `roster-players-fragment.html`
- [ ] Enregistrer les routes dans le router `team_creation`
- [ ] Test E2E : sélection roster → staff affiché → buy → remove → cart mis à jour
