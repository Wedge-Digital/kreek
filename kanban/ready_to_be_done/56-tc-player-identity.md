# BC `team_creation` — Identité des joueurs (nom + numéro de maillot)

**Priorité : haute**
**Dépend de :** infrastructure `team_creation` existante
**Contexte :** `team_creation` — action coach

## Objectif

Permettre au coach de nommer ses joueurs et leur attribuer un numéro de maillot pendant la phase de finalisation. La saisie est non-bloquante (nom facultatif à ce stade) et s'auto-sauvegarde sur `blur`.

---

## Conception

### Value objects

```rust
// team_creation/domain/model/player_identity.rs

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerName(pub String);

impl PlayerName {
    pub fn new(s: &str) -> Result<Self, DomainError> {
        let s = s.trim();
        if s.len() > 50 { return Err(DomainError::PlayerNameTooLong); }
        Ok(Self(s.to_string()))
    }

    pub fn anonymous() -> Self { Self(String::new()) }
    pub fn is_set(&self) -> bool { !self.0.is_empty() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct JerseyNumber(pub u8);

impl JerseyNumber {
    pub fn new(n: u8) -> Result<Self, DomainError> {
        if n < 1 || n > 99 { return Err(DomainError::InvalidJerseyNumber); }
        Ok(Self(n))
    }
}
```

### Événement domaine

```rust
// team_creation/domain/events/
pub struct PlayerIdentitySet {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
    pub name:      PlayerName,
    pub jersey:    JerseyNumber,
}
```

### Commande et use case

```rust
pub struct SetPlayerIdentityCommand {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
    pub name:      PlayerName,     // peut être vide (PlayerName::anonymous())
    pub jersey:    JerseyNumber,
}
```

```rust
pub enum SetPlayerIdentityError {
    TeamNotFound,
    PlayerNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:       SetPlayerIdentityCommand,
    team_repo: &dyn ITeamCreationRepository,
) -> Result<(), SetPlayerIdentityError>
```

Logique :
1. Charger le draft via `team_repo.find_by_id(&cmd.team_id)`
2. Valider `name` et `jersey` (déjà validés par les value objects à la construction de la commande)
3. Vérifier l'unicité du numéro de maillot parmi les joueurs de l'équipe — erreur bloquante (`DomainError::DuplicateJerseyNumber`)
4. Appender `PlayerIdentitySet` + mettre à jour la projection du draft
5. Persister

### Route et handler

```
POST /team-creation/{draft_id}/players/{player_id}/identity
Body : { "name": "Aerindel", "jersey": 7 }
```

**Réponse succès** — deux fragments OOB systématiques :

Fragment 1 : ligne joueur mise à jour dans le panneau gauche (`#player-row-{player_id}`)
Fragment 2 : header joueur dans le panneau droit (`#skill-header`) — toujours retourné, qu'un joueur soit sélectionné ou non. Si aucun joueur n'est sélectionné, le fragment est dans son état vide, ce qui est inoffensif.

**Réponse erreur (HTTP 422)** — fragment inline sous le champ concerné :
- `DomainError::PlayerNameTooLong` → "Le nom ne peut pas dépasser 50 caractères."
- `DomainError::InvalidJerseyNumber` → "Le numéro de maillot doit être compris entre 1 et 99."
- `DomainError::DuplicateJerseyNumber` → "Ce numéro de maillot est déjà attribué à un autre joueur."

### Auto-save dans le template

Les inputs dans le panneau gauche et dans le header du panneau droit partagent le même endpoint :

```html
<!-- Panneau gauche — player-row -->
<input class="jersey-input" type="number"
       name="jersey" value="{{ player.jersey }}"
       hx-post="{{ routes.set_player_identity(draft_id, player.id) }}"
       hx-trigger="change"
       hx-include="closest .player-row">

<input class="player-name-input" type="text"
       name="name" value="{{ player.name }}" placeholder="Nom du joueur"
       hx-post="{{ routes.set_player_identity(draft_id, player.id) }}"
       hx-trigger="blur"
       hx-include="closest .player-row">
```

Les deux inputs dans le header du panneau droit (carte D) pointent vers le même endpoint.

---

## Points à préciser


---

## Checklist

- [ ] `PlayerName` + `JerseyNumber` value objects avec smart constructors
- [ ] `DomainError::PlayerNameTooLong` + `DomainError::InvalidJerseyNumber` + `DomainError::DuplicateJerseyNumber`
- [ ] `PlayerIdentitySet` event dans le domaine `team_creation`
- [ ] `SetPlayerIdentityCommand` + use case (validation + unicité jersey + persist)
- [ ] Route `SET_PLAYER_IDENTITY` dans `routes.rs` + `router.rs`
- [ ] Handler POST : fragments OOB `#player-row-{id}` + `#skill-header` (systématique)
- [ ] Template `player-row-fragment.html` mis à jour (jersey + nom)
- [ ] Template `skill-header-fragment.html` mis à jour (carte D)
- [ ] Inputs `hx-trigger="blur"` / `hx-trigger="change"` câblés dans les templates
