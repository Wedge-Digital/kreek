# Achat et retrait de staff (table section 4)

**Priorité : haute**
**Dépend de :** `21-hire-player.md`, `23-staff-table-roster-update.md`
**Contexte :** `team_creation`

## Objectif

Câbler les boutons `+` / `−` du tableau staff (section 4) sur les méthodes domaine existantes.  
Couvre deux sous-domaines distincts côté domaine :

| Ligne | Méthode domaine | Signature retour |
|---|---|---|
| Staff (cheerleaders, assistant, apothicaire) | `buy_staff` / `remove_staff` | `Result<(), Vec<DomainError>>` / `Result<(), DomainError>` |
| Relances | `purchase_reroll` / `remove_reroll` | `Result<(), Vec<DomainError>>` / `()` (infaillible) |

---

## État de l'existant

| Élément | Fichier | Remarque |
|---|---|---|
| `buy_staff(staff)` | `team_roster_selected.rs:184` | Valide quota, budget ET autorisation roster (toutes les erreurs accumulées dans `Vec`) |
| `remove_staff(staff)` | `team_roster_selected.rs:196` | `DomainError::StaffNotPurchased` si absent |
| `purchase_reroll(count)` | `team_roster_selected.rs:223` | Valide `MaxRerollsExceeded` + `InsufficientRerollBudget` (accumulés) |
| `remove_reroll(count)` | `team_roster_selected.rs:234` | Infaillible — sature à 0 |
| Port de persistance | `ports.rs` → `ITeamRosterRepository` | Créé en carte 21 |

---

## Conception

### Commandes

```rust
// use_cases/commands.rs
pub struct BuyStaffCommand {
    pub team_id:  TeamId,
    pub staff_id: StaffId,   // id du TeamStaff dans Roster.allowed_staff
}

pub struct RemoveStaffCommand {
    pub team_id:  TeamId,
    pub staff_id: StaffId,
}

pub struct BuyRerollCommand {
    pub team_id: TeamId,
}

pub struct RemoveRerollCommand {
    pub team_id: TeamId,
}
```

### Use cases

**`buy_staff.rs`**

```rust
pub enum BuyStaffError {
    TeamNotFound,
    StaffNotFoundInRoster,        // staff_id inconnu dans Roster.allowed_staff
    Domain(Vec<DomainError>),     // accumulés depuis buy_staff()
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:       BuyStaffCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, BuyStaffError>
```

Logique :
1. Charger `RosterSelectedTeam` depuis repo
2. Retrouver `TeamStaff` dans `team.roster.allowed_staff` par `cmd.staff_id` → `StaffNotFoundInRoster`
3. `team.buy_staff(staff_def)` → mapper `Vec<DomainError>` → `BuyStaffError::Domain`
4. Persister + retourner l'équipe mise à jour

**`remove_staff.rs`** — symétrique, `team.remove_staff(&staff_def)` → `DomainError::StaffNotPurchased`.

**`buy_reroll.rs`**

```rust
pub enum BuyRerollError {
    TeamNotFound,
    Domain(Vec<DomainError>),    // MaxRerollsExceeded, InsufficientRerollBudget
    Repository(RepositoryError),
}

pub async fn execute(cmd, team_repo) -> Result<RosterSelectedTeam, BuyRerollError>
```

Logique : `team.purchase_reroll(1)` → persister.

**`remove_reroll.rs`** — `team.remove_reroll(1)` est infaillible. Peut échouer uniquement sur `TeamNotFound` ou repo.

---

### Routes

```
POST /app/{space_id}/team/{team_id}/staff/buy     body: { "staff_id": "apothecary" }
POST /app/{space_id}/team/{team_id}/staff/remove  body: { "staff_id": "apothecary" }
POST /app/{space_id}/team/{team_id}/rerolls/buy
POST /app/{space_id}/team/{team_id}/rerolls/remove
```

Dans `routes.rs` : 4 nouvelles constantes + méthodes helper.

### Pattern HTMX (identique cartes 21/22)

- **200** : fragment `<tr id="staff-row-{id}">` mis à jour + OOB swap `#build-cart`
- **422** : fragment erreur `<div id="staff-error-{id}">` ciblé sur la ligne concernée

Bouton `+` désactivé si `quantity >= max_qty`. Bouton `−` désactivé si `quantity == 0`.

```html
<!-- Relances -->
<button type="button"
        hx-post="{{ team_routes.buy_reroll(space_id, team_id) }}"
        hx-target="closest tr" hx-swap="outerHTML"
        {% if reroll_count >= 8 %}disabled{% endif %}>+</button>

<!-- Staff standard -->
<button type="button"
        hx-post="{{ team_routes.buy_staff(space_id, team_id) }}"
        hx-ext="json-enc"
        hx-vals='{"staff_id":"{{ row.id }}"}'
        hx-target="closest tr" hx-swap="outerHTML"
        {% if row.quantity >= row.max_qty %}disabled{% endif %}>+</button>
```

### Mapping erreurs → messages FR

| `DomainError` | Message |
|---|---|
| `StaffMaxReached` | Vous avez atteint le nombre maximum pour ce poste. |
| `StaffNotAllowed` | Ce poste n'est pas disponible pour ce roster. |
| `StaffNotPurchased` | Ce poste n'a pas été acheté. |
| `InsufficientBudget` | Budget insuffisant pour ce poste. |
| `MaxRerollsExceeded` | Vous ne pouvez pas prendre plus de 8 relances. |
| `InsufficientRerollBudget` | Budget insuffisant pour cette relance. |

---

## Checklist

- [ ] `BuyStaffCommand`, `RemoveStaffCommand`, `BuyRerollCommand`, `RemoveRerollCommand` dans `commands.rs`
- [ ] Use cases `buy_staff.rs`, `remove_staff.rs`, `buy_reroll.rs`, `remove_reroll.rs`
- [ ] 4 routes dans `routes.rs` + `router.rs`
- [ ] 4 handlers POST avec réponses 200 (OOB) / 422 (erreur)
- [ ] Template `staff-row-fragment.html` dans `references/io/web/templates/`
- [ ] Boutons `+`/`−` dans `roster-staff-fragment.html` (carte 23) câblés avec les bons `hx-*`
- [ ] Gestion de l'état `disabled` pour tous les boutons selon quantité et limites