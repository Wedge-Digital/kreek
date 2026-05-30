# Renvoyer un joueur (bouton − de la table)

**Priorité : haute**
**Dépend de :** `21-hire-player.md`
**Contexte :** `team_creation`

## Objectif

Clic sur `−` dans la table joueurs → commande `FirePlayer` → `RosterSelectedTeam.remove_player()` → persistance → mise à jour HTMX de la ligne joueur et du panier. Opération symétrique de la carte 21.

---

## État de l'existant

| Élément | Fichier | Remarque |
|---|---|---|
| Logique domaine | `domain/team_roster_selected.rs` → `remove_player()` | Valide que le joueur est bien embauché (`PlayerNotHired`) |
| Port de persistance | `ports.rs` → `ITeamRosterRepository` | Créé en carte 21 |

---

## Conception

### Commande et use case

```rust
// use_cases/commands.rs
pub struct FirePlayerCommand {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
}
```

```rust
// use_cases/fire_player.rs
pub enum FirePlayerError {
    TeamNotFound,
    PlayerNotFound,
    Domain(DomainError),       // PlayerNotHired
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:       FirePlayerCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, FirePlayerError>
```

Logique :
1. `team_repo.find_by_id(&cmd.team_id)` → `TeamNotFound` si absent
2. Retrouver `PlayerDefinition` dans `team.roster.player_definitions` par `cmd.player_id`
3. `team.remove_player(&player_def)` → mapper `DomainError::PlayerNotHired`
4. `team_repo.save(&team, space_id)`
5. Retourner `team` mis à jour

Le bouton `−` doit être désactivé (attribut `disabled`) quand `quantity == 0` pour éviter les appels inutiles — mais le handler retourne 422 + fragment erreur si appelé quand même.

### Route et handler

```
POST /app/{space_id}/team/{team_id}/players/fire
Body JSON : { "player_id": "marauder-lineman" }
```

Même pattern de réponse que la carte 21 :
- **200** : fragment `<tr id="player-row-{uid}">` mis à jour + OOB swap `#build-cart`
- **422** : fragment erreur ciblé

### Comportement bouton `−`

```html
<button type="button"
        hx-post="{{ team_routes.fire_player(space_id, team_id) }}"
        hx-ext="json-enc"
        hx-vals='{"player_id":"{{ pos.uid }}"}'
        hx-target="closest tr"
        hx-swap="outerHTML"
        {% if pos.quantity == 0 %}disabled{% endif %}>−</button>
```

---

## Checklist

- [ ] `FirePlayerCommand` dans `commands.rs`
- [ ] Use case `fire_player.rs` avec `FirePlayerError`
- [ ] Route `FIRE_PLAYER` dans `routes.rs` + `router.rs`
- [ ] Handler POST `fire_player` : 200 fragments OOB, 422 fragment erreur
- [ ] Boutons `−` dans `build-team.html` câblés + `disabled` si `quantity == 0`