# Engager un joueur (bouton + de la table)

**Priorité : haute**
**Dépend de :** `20-build-team-roster-wiring.md`
**Contexte :** `team_creation`

## Objectif

Clic sur `+` dans la table joueurs (section 3) → commande `HirePlayer` → validation domaine via `RosterSelectedTeam.hire_player()` → persistance → mise à jour HTMX de la ligne joueur et du panier.

---

## État de l'existant

| Élément | Fichier | Remarque |
|---|---|---|
| Logique domaine | `domain/team_roster_selected.rs` → `hire_player()` | Complet. Valide budget, quotas, cross-limits |
| Erreurs domaine | `domain/error.rs` | `MaxPlayersReached`, `InsufficientBudget`, `CrossLimitExceeded`, … |
| `PlayerDefinition` | `domain/roster.rs` | `{ id, name, max_quantity, price }` |
| Commande | `use_cases/commands.rs` → `ChooseRosterCommand` | Modèle existant à dupliquer |
| Port actuel | `ports.rs` → `ITeamDraftRepository` | Couvre uniquement `DraftTeam` — `RosterSelectedTeam` non persisté |

---

## Conception

### Nouveau port de persistance

`RosterSelectedTeam` est l'agrégat principal pendant la construction. Il nécessite son propre port.

```rust
// team_creation/ports.rs
#[async_trait]
pub trait ITeamRosterRepository: Send + Sync {
    async fn save(&self, team: &RosterSelectedTeam, space_id: &str) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &TeamId)  -> Result<Option<RosterSelectedTeam>, RepositoryError>;
}
```

Implémentation : nouvelle table `team_roster_selections`, colonnes `id`, `space_id`, `state` (JSON de l'agrégat complet), `created_at`, `updated_at`. Migration à créer.

> `RosterSelectedTeam` est déjà `Serialize/Deserialize` — sérialisation JSON directe.

### Commande et use case

```rust
// use_cases/commands.rs
pub struct HirePlayerCommand {
    pub team_id:   TeamId,
    pub player_id: PlayerId,  // uid du poste dans le roster
}
```

```rust
// use_cases/hire_player.rs
pub enum HirePlayerError {
    TeamNotFound,
    PlayerNotFound,       // uid inconnu dans le roster courant
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:        HirePlayerCommand,
    team_repo:  &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, HirePlayerError>
```

Logique :
1. `team_repo.find_by_id(&cmd.team_id)` → `TeamNotFound` si absent
2. Retrouver `PlayerDefinition` depuis `team.roster.player_definitions` par `cmd.player_id`
3. `team.hire_player(player_def)` → mapper `DomainError` → `HirePlayerError::Domain`
4. `team_repo.save(&team, space_id)` → persister
5. Retourner `team` mis à jour

### Route et handler

```
POST /app/{space_id}/team/{team_id}/players/hire
Body JSON : { "player_id": "marauder-lineman" }
```

Handler retourne deux fragments HTMX :

**Fragment principal** — ligne joueur mise à jour (`<tr id="player-row-{uid}">`), remplaçant la ligne cliquée via `hx-target="closest tr"` :

```html
<tr id="player-row-{{ pos.uid }}">
  <td>{{ pos.max_qty_label }}</td>
  <td class="player-name">{{ pos.name }}</td>
  <td>{{ pos.cost }} kPo</td>
  …
  <td>{{ pos.quantity }}</td>
  <td>{{ pos.line_cost }} kPo</td>
  <td><button hx-post="…/hire" hx-vals='{"player_id":"{{ pos.uid }}"}' …>+</button></td>
  <td><button hx-post="…/fire" hx-vals='{"player_id":"{{ pos.uid }}"}' …>−</button></td>
</tr>
```

**OOB swap** — panier mis à jour (`hx-swap-oob="true"` sur `#build-cart`) :

```html
<div id="build-cart" hx-swap-oob="true">
  … totaux recalculés …
</div>
```

En cas d'erreur domaine, le handler retourne `HTTP 422` avec un fragment :

```html
<div id="player-error-{{ uid }}" class="player-row-error">
  Budget insuffisant pour ce joueur.
</div>
```

ciblé via `hx-target="#player-error-{{ uid }}"` sur le bouton.

### Mapping `DomainError` → message utilisateur

| `DomainError` | Message FR |
|---|---|
| `MaxPlayersReached` | Vous ne pouvez pas engager plus de 16 joueurs. |
| `MaxPlayersOfTypeReached` | Quota maximum de ce poste atteint. |
| `InsufficientBudget` | Budget insuffisant pour ce joueur. |
| `CrossLimitExceeded` | Limite combinée de postes dépassée. |
| `PlayerNotAllowedInRoster` | Ce poste n'est pas disponible dans ce roster. |

### Boutons dans la table (build-team.html)

```html
<button type="button"
        hx-post="{{ team_routes.hire_player(space_id, team_id) }}"
        hx-ext="json-enc"
        hx-vals='{"player_id":"{{ pos.uid }}"}'
        hx-target="closest tr"
        hx-swap="outerHTML"
        hx-on::after-request="refreshCart(this)">+</button>
```

---

## Checklist

- [ ] Migration `team_roster_selections` (id, space_id, state JSONB, created_at, updated_at)
- [ ] `ITeamRosterRepository` dans `ports.rs`
- [ ] Implémentation `TeamRosterRepository` dans `io/team_creation_repository.rs`
- [ ] `TeamCreationContext` expose le nouveau repository ; `AppState` mis à jour
- [ ] `HirePlayerCommand` dans `commands.rs`
- [ ] Use case `hire_player.rs` avec `HirePlayerError`
- [ ] Route `HIRE_PLAYER` dans `routes.rs` + `router.rs`
- [ ] Handler POST `hire_player` : 200 avec fragments OOB, 422 avec fragment erreur
- [ ] Templates : `player-row-fragment.html` + `build-cart-fragment.html`
- [ ] Boutons `+` dans `build-team.html` câblés avec les bons attributs HTMX
- [ ] Chaque `<tr>` de la table a `id="player-row-{uid}"`