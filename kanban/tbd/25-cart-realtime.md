# Panier temps réel — fragment OOB partagé

**Priorité : haute**
**Dépend de :** `21-hire-player.md`, `22-fire-player.md`, `24-buy-remove-staff.md`
**Contexte :** `team_creation`

## Objectif

Définir le fragment `#build-cart` retourné en OOB swap par toutes les actions de construction (embauche / renvoi joueur, achat / retrait staff, achat / retrait relance). Le panier est entièrement calculé côté serveur depuis `RosterSelectedTeam` — aucun calcul JS.

---

## État de l'existant

Le `<div class="cart">` dans `build-team.html` est 100 % hardcodé. Les cartes 21-24 mentionnent un "OOB swap `#build-cart`" sans en définir le contenu.

Structure actuelle du panier :
- Lignes joueurs (nom, `×N`, coût total)
- Sous-total joueurs
- Lignes staff + relances
- Sous-total staff
- Total général
- Barre de budget (utilisé / max, % progression, kPo restants)
- Bouton "Valider →"

---

## Conception

### Vue model

```rust
// team_creation/io/web/build_team.rs (ou cart_vm.rs dédié)

pub struct CartLine {
    pub name:      String,
    pub quantity:  u32,
    pub line_cost: u32,
}

pub struct BuildCartVm {
    pub player_lines:    Vec<CartLine>,  // positions avec qty > 0, groupées par id
    pub player_subtotal: u32,
    pub staff_lines:     Vec<CartLine>,  // relances en premier (si qty > 0), puis staff
    pub staff_subtotal:  u32,
    pub total:           u32,
    pub budget:          u32,
    pub remaining:       u32,
    pub budget_pct:      u8,            // min(total * 100 / budget, 100)
}

pub fn build_cart_vm(team: &RosterSelectedTeam) -> BuildCartVm {
    // 1. Grouper hired_players par PlayerDefinition.id
    //    — conserver l'ordre des player_definitions du roster (tri stable)
    // 2. Relances en tête des staff_lines si reroll_count > 0
    //    — nom "Relances", price = roster.reroll_price.0 * count
    // 3. Grouper hired_staff par StaffId
    // 4. budget depuis team.remaining_budget() — unwrap_or(0) (ne peut pas échouer en flux normal)
    // 5. remaining = budget.saturating_sub(total)
    // 6. budget_pct = if budget == 0 { 0 } else { ((total * 100) / budget).min(100) as u8 }
}
```

### Template fragment

Nouveau fichier : `team_creation/io/web/templates/build-cart-fragment.html`

```html
<div id="build-cart" class="cart" hx-swap-oob="true">
  <div class="cart-title">🛒 Récapitulatif</div>

  {% if !cart.player_lines.is_empty() %}
  <div class="cart-section-label">Joueurs</div>
  {% for line in cart.player_lines %}
  <div class="cart-line">
    <span class="cart-line-name">{{ line.name }}</span>
    <span class="cart-line-qty">×{{ line.quantity }}</span>
    <span class="cart-line-price">{{ line.line_cost }} kPo</span>
  </div>
  {% endfor %}
  <div class="cart-subtotal">
    <span>Sous-total joueurs</span>
    <span>{{ cart.player_subtotal }} kPo</span>
  </div>
  <hr class="cart-separator">
  {% endif %}

  {% if !cart.staff_lines.is_empty() %}
  <div class="cart-section-label">Staff &amp; équipement</div>
  {% for line in cart.staff_lines %}
  <div class="cart-line">
    <span class="cart-line-name">{{ line.name }}</span>
    <span class="cart-line-qty">×{{ line.quantity }}</span>
    <span class="cart-line-price">{{ line.line_cost }} kPo</span>
  </div>
  {% endfor %}
  <div class="cart-subtotal">
    <span>Sous-total staff</span>
    <span>{{ cart.staff_subtotal }} kPo</span>
  </div>
  <hr class="cart-separator">
  {% endif %}

  {% if cart.total == 0 %}
  <div style="color: var(--dark-3); font-style: italic; font-size: var(--text-small); text-align: center; padding: var(--p2);">
    Aucun achat pour le moment.
  </div>
  {% else %}
  <div class="cart-total">
    <span class="cart-total-label">Total</span>
    <span class="cart-total-amount">{{ cart.total }} kPo</span>
  </div>
  {% endif %}

  <div class="cart-budget-bar">
    <div class="cart-budget-meta">
      <span>Budget utilisé</span>
      <span>{{ cart.total }} / {{ cart.budget }} kPo</span>
    </div>
    <div class="cart-progress">
      <div class="cart-progress-inner" style="width: {{ cart.budget_pct }}%;"></div>
    </div>
    <div style="font-size: var(--text-tiny); font-weight: 600; text-align: right;
                color: {% if cart.remaining == 0 %}var(--orange){% else %}var(--green){% endif %};">
      {% if cart.remaining == 0 %}Budget épuisé{% else %}{{ cart.remaining }} kPo restants{% endif %}
    </div>
  </div>

  <div class="cart-actions">
    <button class="btn btn-primary"
            {% if cart.total == 0 %}disabled{% endif %}>Valider →</button>
  </div>
</div>
```

### Fonction helper partagée

Pour éviter la duplication dans chaque handler (21, 22, 24), extraire un helper :

```rust
// team_creation/io/web/build_team.rs
pub fn render_cart_oob(team: &RosterSelectedTeam) -> String {
    let vm = build_cart_vm(team);
    BuildCartFragment { cart: vm }.render().unwrap_or_default()
}
```

Chaque handler appelle `render_cart_oob(&team)` et concatène au fragment principal avant de retourner `Html(format!("{main_fragment}{cart_oob}"))`.

### Réinitialisation au changement de roster

Quand un nouveau roster est sélectionné (carte 20), `RosterSelectedTeam.choose_roster()` efface tous les achats. Le handler `get_roster_players` doit retourner :
1. `<tbody id="player-table-body">` — joueurs du nouveau roster
2. `<tbody id="staff-table-body" hx-swap-oob="true">` — staff du nouveau roster (carte 23)
3. `<div id="build-cart" hx-swap-oob="true">` — panier vide

Le panier vide correspond à `build_cart_vm` appelé sur le `RosterSelectedTeam` fraîchement créé (zéro achats) avec le budget du tier.

### Panier initial au chargement de la page

Si l'équipe existe déjà en base avec des achats (reprise de session), le template `build-team.html` doit inclure le panier rendu côté serveur dès le premier GET. `BuildTeamTemplate` reçoit un champ `cart: BuildCartVm` calculé depuis le `RosterSelectedTeam` chargé.

---

## Checklist

- [ ] `CartLine` + `BuildCartVm` + `build_cart_vm()` dans `build_team.rs` (ou `cart_vm.rs`)
- [ ] Template `build-cart-fragment.html` dans `team_creation/io/web/templates/`
- [ ] Helper `render_cart_oob()` réutilisé par les handlers 21, 22, 24
- [ ] Handler `get_roster_players` (carte 20) inclut le panier vide en OOB au changement de roster
- [ ] `BuildTeamTemplate` reçoit `cart: BuildCartVm` pour le rendu initial
- [ ] `<div class="cart">` dans `build-team.html` remplacé par inclusion du fragment (ou `id="build-cart"` ajouté)
- [ ] Bouton "Valider →" désactivé si `cart.total == 0`