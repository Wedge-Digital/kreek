# BC `players` — Structure & Agrégat

**Priorité : haute**
**Dépend de :** rien — carte fondatrice du BC
**Contexte :** nouveau BC `players`

## Objectif

Poser les bases du BC `players` : module, types domaine, agrégat `Player` event-sourcé,
et la définition de `PlayerCreated`, l'unique domain event de la version minimale.

---

## Structure du module

```
src/app/players/
├── mod.rs
├── context.rs              # PlayersContext (repositories, etc.)
├── domain/
│   ├── mod.rs
│   ├── player.rs           # agrégat Player
│   ├── events.rs           # PlayerDomainEvent enum
│   └── error.rs            # DomainError
├── ports.rs                # IPlayerRepository
└── io/
    ├── mod.rs
    └── repository/         # implémentation Postgres (carte 62)
```

---

## Value objects

```rust
// Réutiliser l'instance_id de team_creation comme identité canonique
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamId(pub String);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Spp(pub u32);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ValueKpo(pub u32);
```

---

## Domain event (version minimale)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerDomainEvent {
    PlayerCreated {
        player_id:       PlayerId,
        team_id:         TeamId,
        space_id:        String,
        position_name:   String,
        roster_line_id:  String,       // UID de position dans le référentiel
        personal_name:   String,       // vide si non saisi
        jersey:          Option<u8>,
        base_skills:     Vec<String>,  // UIDs de compétences du poste
        acquired_skills: Vec<AcquiredSkill>,
        starting_spp:    Spp,          // 0 pour la plupart ; SPP hors-finalization si compétition spéciale
        starting_value:  ValueKpo,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquiredSkill {
    pub skill_id: String,
    pub mode:     AcquisitionMode,  // Chosen | Random
    pub spp_cost: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AcquisitionMode { Chosen, Random }
```

---

## Agrégat Player

```rust
pub struct Player {
    pub id:              PlayerId,
    pub team_id:         TeamId,
    pub position_name:   String,
    pub roster_line_id:  String,
    pub personal_name:   String,
    pub jersey:          Option<u8>,
    pub base_skills:     Vec<String>,
    pub acquired_skills: Vec<AcquiredSkill>,
    pub spp:             Spp,
    pub value:           ValueKpo,
}

impl Player {
    pub fn apply(event: &PlayerDomainEvent) -> Option<Self> { ... }
    // retourne None si aucun event — pas d'état "vide" explicite
}
```

`apply()` reconstruit l'état depuis les events (pattern event sourcing : pas de `new()` direct,
on ne crée un Player qu'en rejouant ses events depuis le store).

---

## Calcul de `starting_value`

```
starting_value = position_base_cost
               + Σ spp_cost × coeff(mode, access)
```

Les coefficients sont les prix du référentiel `skill_cost.json` (level 1 à la création).
Ce calcul est effectué par le use case (carte 64) au moment de construire `PlayerCreated`,
pas dans le domaine.

---

## Checklist

- [ ] `src/app/players/` créé avec structure complète
- [ ] Value objects `PlayerId`, `TeamId`, `Spp`, `ValueKpo`
- [ ] `AcquiredSkill`, `AcquisitionMode` (peut être mutualisé avec `shared_kernel`)
- [ ] `PlayerDomainEvent::PlayerCreated` avec tous les champs
- [ ] `Player::apply()` implémenté
- [ ] Câblage dans `src/app/mod.rs` et `main.rs` (context vide pour l'instant)
