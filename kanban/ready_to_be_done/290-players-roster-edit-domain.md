# BC `players` — Domaine : renommage, numéro, ordre des joueurs

**Priorité : haute**
**Dépend de :** —
**Contexte :** `players` — domaine

## Objectif

Poser les fondations domaine de l'édition d'effectif (renommer un joueur,
changer son numéro de maillot, le réordonner) : value objects, nouveaux
champs sur l'agrégat `Player`, événements, méthodes de mutation, erreurs.
Aucun handler ni use case dans cette carte — pur domaine, testable en
isolation.

**Spec de référence :** `docs/specs/player-edition/team-detail/06-domaine.md`.

---

## Conception

### Value objects (`players/domain/value_objects.rs`)

```rust
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = r"^[\p{L}0-9 '-]+$"),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct PersonalName(String);

#[nutype(
    derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)
)]
pub struct DisplayOrder(u32);
```

`JerseyVo` (existant, ligne 40) : `validate(less_or_equal = 999)` devient
`validate(greater_or_equal = 1, less_or_equal = 99)`. Vérifier qu'aucun
appelant existant (création de joueur côté `team_creation` → `players` via
app event) ne passe une valeur hors `1..99` — les valeurs viennent
aujourd'hui toujours de `team_creation::JerseyNumber` (déjà `1..16`), donc
sans risque.

### Agrégat `Player` — nouveaux champs

```rust
pub struct Player {
    // ... champs existants inchangés ...
    pub personal_name: Option<PersonalName>,
    pub jersey: Option<JerseyVo>,            // existant, inchangé
    pub display_order: Option<DisplayOrder>,
}
```

### Événements (`players/domain/events.rs`)

```rust
PlayerRenamed { player_id: PlayerId, team_id: TeamId, personal_name: Option<PersonalName> },
PlayerJerseyChanged { player_id: PlayerId, team_id: TeamId, jersey: Option<JerseyVo> },
PlayerReordered { player_id: PlayerId, team_id: TeamId, display_order: DisplayOrder },
```

`apply()` gagne une branche par variante, folding le champ correspondant.

### Méthodes de domaine (`players/domain/player.rs`)

```rust
impl Player {
    pub fn rename(&self, personal_name: Option<PersonalName>) -> Result<PlayerDomainEvent, DomainError> { ... }
    pub fn change_jersey(&self, jersey: Option<JerseyVo>) -> Result<PlayerDomainEvent, DomainError> { ... }
    pub fn reorder(&self, display_order: DisplayOrder) -> Result<PlayerDomainEvent, DomainError> { ... }
    fn guard_active(&self) -> Result<(), DomainError> { ... } // membership == Active sinon PlayerNotActive
}
```

Aucune méthode ne mute `self` en place (cohérent avec `increase_stat`
existant) : calcule et retourne l'événement, ne persiste rien.

### `DomainError` (`players/domain/error.rs`)

Nouvelle variante `PlayerNotActive` ("joueur non actif").

---

## Checklist

- [ ] `PersonalName` (nutype)
- [ ] `DisplayOrder` (nutype)
- [ ] `JerseyVo` resserré à `1..99`
- [ ] Champs `personal_name`/`display_order` sur `Player`
- [ ] Événements `PlayerRenamed`/`PlayerJerseyChanged`/`PlayerReordered`
- [ ] `apply()` : 3 nouvelles branches
- [ ] Méthodes `rename`/`change_jersey`/`reorder` + `guard_active()`
- [ ] `DomainError::PlayerNotActive`
- [ ] Tests : `rename_produces_player_renamed_event_with_new_name`
- [ ] Tests : `rename_allows_clearing_to_none`
- [ ] Tests : `change_jersey_produces_player_jersey_changed_event`
- [ ] Tests : `change_jersey_allows_clearing_to_none`
- [ ] Tests : `reorder_produces_player_reordered_event`
- [ ] Tests : `rename_change_jersey_reorder_reject_dismissed_player`
- [ ] Tests : `personal_name_rejects_over_50_chars`
- [ ] Tests : `personal_name_allows_apostrophe`
- [ ] Tests : `personal_name_rejects_empty_string`
- [ ] Tests : `jersey_vo_rejects_zero_and_above_99`
