# BC `team_creation` — Widget Cart autonome

**Priorité : haute (bloquant pour 70, 71)**
**Dépend de :** `67-tc-vm-rapatriement.md`
**Contexte :** BC `team_creation` — widget HTMX

## Objectif

Transformer le cart (panier récapitulatif) en widget autonome avec son propre endpoint. Le cart se recharge via un événement DOM `teamMutated` au lieu d'être inclus en OOB swap dans chaque réponse de mutation.

---

## Situation actuelle

- Le cart est rendu par une macro Askama `cart_body()` dans `cart-fragment.html`
- Chaque fragment de réponse (player-row, staff-row, reroll-row, roster-players) inclut un `<div id="team-cart" hx-swap-oob="true">{% call cart_cmp::cart_body(cart) %}</div>`
- Le cart VM (`CartVm`) est recalculé dans chaque handler de mutation
- 6 handlers incluent le cart en OOB : `hire_player`, `fire_player`, `buy_staff`, `remove_staff`, `buy_reroll`, `remove_reroll`, `get_roster_players`

---

## Conception

### Nouvel endpoint

```
GET /app/{space_id}/team/{team_id}/widgets/cart
```

### Handler

```rust
pub async fn cart_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let roster_team = state.team_creation.roster_repository
        .find_by_id(&TeamId::try_new(&team_id).unwrap())
        .await;

    let cart = match roster_team {
        Ok(Some(team)) => Some(build_cart_vm(&team)),
        _ => None,
    };

    CartWidgetTemplate { cart }.into_response()
}
```

### Template : `widgets/cart-widget.html`

```html
<div id="team-cart" class="cart" hx-disinherit="*">
  <!-- contenu actuel de cart_body() -->
</div>
```

Le template reprend le contenu de la macro `cart_body()` mais dans un fragment autonome (plus besoin de macro). Respecte la règle widget : `hx-disinherit="*"` à la racine.

### Intégration dans la page hôte `build-team.html`

```html
<div id="team-cart"
     hx-get="{{ team_routes.cart_widget(space_id, team_id) }}"
     hx-trigger="load, teamMutated from:body"
     hx-target="this"
     hx-swap="innerHTML">
</div>
```

### Modification des handlers de mutation

Chaque handler de mutation (hire/fire/buy/remove/reroll) :
1. **Supprime** l'OOB swap du cart dans sa réponse
2. **Ajoute** un header `HX-Trigger: teamMutated` dans sa réponse

```rust
Response::builder()
    .header("HX-Trigger", "teamMutated")
    // ... body du fragment row mis à jour
```

### Nettoyage

- Supprimer la macro `cart_body()` de `cart-fragment.html` (le fichier entier peut être supprimé)
- Supprimer les `{% import "cart-fragment.html" as cart_cmp %}` de tous les fragments row
- Supprimer les `<div id="team-cart" hx-swap-oob="true">` de `roster-players-fragment.html`, `player-row-fragment.html`, `staff-row-fragment.html`, `reroll-row-fragment.html`
- Supprimer le calcul de `cart` dans les handlers de mutation (plus besoin de le passer aux templates de row)

---

## Situation finale

- Le cart a son propre endpoint `GET .../widgets/cart`
- Le cart est chargé au `load` de la page et se recharge automatiquement sur `teamMutated from:body`
- Les fragments de row (player, staff, reroll) ne contiennent plus d'OOB swap cart
- Les handlers de mutation retournent uniquement leur fragment row + `HX-Trigger: teamMutated`
- Le calcul du `CartVm` n'a lieu qu'à un seul endroit (le handler cart_widget)
- La route est déclarée dans `team_creation/routes.rs`

---

## Checklist

- [ ] Ajouter `CART_WIDGET` dans `team_creation/routes.rs` (path + méthode)
- [ ] Créer le handler `cart_widget` dans un nouveau fichier `team_creation/io/web/widgets/cart_widget.rs`
- [ ] Créer le template `team_creation/io/web/templates/widgets/cart-widget.html`
- [ ] Enregistrer la route dans le router `team_creation`
- [ ] Modifier la page `build-team.html` : remplacer le rendu statique du cart par `hx-get` + `hx-trigger="load, teamMutated from:body"`
- [ ] Modifier `hire_player` : supprimer OOB cart, ajouter `HX-Trigger: teamMutated`
- [ ] Modifier `fire_player` : idem
- [ ] Modifier `buy_staff` : idem
- [ ] Modifier `remove_staff` : idem
- [ ] Modifier `buy_reroll` : idem
- [ ] Modifier `remove_reroll` : idem
- [ ] Modifier `get_roster_players` : supprimer OOB cart
- [ ] Supprimer `cart-fragment.html` (macro devenue inutile)
- [ ] Supprimer les imports `cart_cmp` des templates de fragments row
- [ ] Test E2E : vérifier que le cart se met à jour après hire/fire/buy/remove
