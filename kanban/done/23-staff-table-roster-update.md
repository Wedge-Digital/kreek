# Mise à jour du tableau staff en fonction du roster sélectionné

**Priorité : haute**
**Dépend de :** `20-build-team-roster-wiring.md`
**Contexte :** `team_creation` + `references`

## Objectif

Quand un roster est sélectionné (section 2), la table staff (section 4) doit se mettre à jour automatiquement pour refléter :
- Le **prix des relances** propre au roster (`Roster.reroll_price`)
- Les **postes de staff disponibles** pour ce roster (`Roster.allowed_staff`)
- L'absence d'apothicaire pour les rosters qui n'y ont pas droit (`StaffKind::Apothecary` absent de `allowed_staff`)

---

## État de l'existant

| Élément | Fichier | Remarque |
|---|---|---|
| `Roster.reroll_price` | `domain/roster.rs` | `RerollBasePrice(u32)` — varie par roster |
| `Roster.allowed_staff` | `domain/roster.rs` | `Vec<TeamStaff>` — filtré par roster |
| `StaffKind` | `shared_kernel/staff.rs` | `Apothecary \| Cheerleaders \| CoachAssistant` |
| `RosterSelectedTeam` | `domain/team_roster_selected.rs` | `buy_staff()` / `remove_staff()` déjà implémentés |
| Table staff hardcodée | `build-team.html` section 4 | Affiche toujours relances + cheerleaders + assistants + apothicaire |

Note : les relances ne sont **pas** un `TeamStaff` — elles ont leur propre logique (`purchase_reroll` / `remove_reroll`). Le prix est dans `Roster.reroll_price`.

---

## Conception

### Stratégie de mise à jour

Le clic sur un chip roster déclenche déjà `GET .../roster/{uid}/players` (carte 20) qui retourne le `<tbody>` des joueurs. On étend cette réponse avec un **OOB swap HTMX** du `<tbody>` staff, sans changer les attributs du chip.

```
GET /app/{space_id}/team/{team_id}/roster/{roster_uid}/players
→ Response body :
    1. Fragment principal : <tbody id="player-table-body"> (joueurs)
    2. OOB : <tbody id="staff-table-body" hx-swap-oob="true"> (staff)
```

### Vue model

```rust
// references/io/web/pickers.rs
pub struct StaffRowVm {
    pub id:       String,
    pub name:     String,
    pub price:    u32,
    pub max_qty:  u8,
}

pub struct StaffTableVm {
    pub reroll_price: u32,           // depuis Roster.reroll_price.0
    pub staff_rows:   Vec<StaffRowVm>, // depuis Roster.allowed_staff (triés par kind)
}

pub fn build_staff_table(roster: &Roster) -> StaffTableVm {
    let mut rows: Vec<StaffRowVm> = roster.allowed_staff.iter()
        .map(|s| StaffRowVm {
            id:      s.id.0.clone(),
            name:    s.name.0.clone(),
            price:   s.price.0,
            max_qty: s.max_quantity.0,
        })
        .collect();
    // Ordre stable : Cheerleaders, CoachAssistant, Apothecary
    rows.sort_by_key(|r| match r.id.as_str() {
        id if id.contains("cheerleader") => 0,
        id if id.contains("assistant")   => 1,
        _                                => 2,
    });
    StaffTableVm { reroll_price: roster.reroll_price.0, staff_rows: rows }
}
```

### Fragment template

Nouveau fichier : `references/io/web/templates/roster-staff-fragment.html`

```html
<tbody id="staff-table-body" hx-swap-oob="true">
  <!-- Relances — toujours présentes, prix variable -->
  <tr id="staff-row-reroll">
    <td>0-8</td>
    <td style="text-align:left; font-weight:500;">Relances</td>
    <td>{{ staff.reroll_price }} kPo</td>
    <td>0</td>
    <td>0 kPo</td>
    <td><button class="tbl-btn" type="button" …>+</button></td>
    <td><button class="tbl-btn" type="button" disabled>−</button></td>
  </tr>
  <!-- Staff autorisé par ce roster (apothicaire absent si non disponible) -->
  {% for row in staff.staff_rows %}
  <tr id="staff-row-{{ row.id }}">
    <td>0-{{ row.max_qty }}</td>
    <td style="text-align:left; font-weight:500;">{{ row.name }}</td>
    <td>{{ row.price }} kPo</td>
    <td>0</td>
    <td>0 kPo</td>
    <td><button class="tbl-btn" type="button" …>+</button></td>
    <td><button class="tbl-btn" type="button" disabled>−</button></td>
  </tr>
  {% endfor %}
</tbody>
```

### Modifications du handler `get_roster_players` (carte 20)

Le handler construit désormais les deux fragments et les concatène :

```rust
let player_fragment = /* render roster-players-fragment.html */;
let staff_vm  = build_staff_table(&team_roster);
let staff_fragment  = /* render roster-staff-fragment.html with staff_vm */;

Html(format!("{player_fragment}{staff_fragment}")).into_response()
```

### Modifications de `build-team.html`

- `<tbody>` de la table staff reçoit `id="staff-table-body"`
- Contenu initial : message d'état vide (aucun roster sélectionné)
- Les boutons `+`/`−` du staff seront câblés dans une carte ultérieure (buy/remove staff)

### État initial (page chargée sans roster)

```html
<tbody id="staff-table-body">
  <tr>
    <td colspan="7" style="text-align:center; color:var(--dark-3); font-style:italic;">
      Sélectionnez un roster pour afficher le staff disponible.
    </td>
  </tr>
</tbody>
```

---

## Checklist

- [ ] `StaffRowVm` + `StaffTableVm` + `build_staff_table()` dans `references/io/web/pickers.rs`
- [ ] Template `roster-staff-fragment.html` dans `references/io/web/templates/`
- [ ] Handler `get_roster_players` étendu : retourne joueurs + OOB staff dans la même réponse
- [ ] `build-team.html` section 4 : `<tbody id="staff-table-body">` + état vide initial
- [ ] Vérifier que les IDs de lignes (`staff-row-reroll`, `staff-row-{id}`) sont stables pour les futures cartes buy/remove staff