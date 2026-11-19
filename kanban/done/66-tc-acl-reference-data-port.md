# BC `team_creation` — Anti-Corruption Layer : port de données de référence

**Priorité : haute (bloquant pour 67–72)**
**Dépend de :** rien
**Contexte :** BC `team_creation` — ports + infrastructure

## Objectif

Éliminer les imports directs de `crate::app::references` dans le BC `team_creation` en introduisant un port (trait) qui abstrait l'accès aux données de référence. L'implémentation de ce port vit dans la couche infrastructure et est injectée dans le `TeamCreationContext` au niveau de `main.rs`.

---

## Situation actuelle

`build_team.rs` importe directement :
- `crate::app::references::domain::models::Team as RefTeam`
- `crate::app::references::domain::port::IReferenceRepository`
- `crate::app::references::io::web::pickers::{build_player_positions, build_roster_items_with_tiers, HiredPlayerRowVm, PlayerPositionVm, RosterPickerItemWithTier}`

Les handlers appellent `state.references.repository.as_ref()` dans 5 endroits.

---

## Conception

### Port (trait) dans `team_creation/ports.rs`

```rust
// team_creation/ports.rs

/// Données de référence nécessaires à la construction d'équipe.
/// L'implémentation est fournie par l'infrastructure (pas par le BC references directement).
pub trait IReferenceDataPort: Send + Sync {
    /// Retourne la définition d'un roster par son UID.
    fn find_roster_definition(&self, roster_uid: &str) -> Option<RosterDefinition>;

    /// Liste les définitions de staff disponibles.
    fn list_staff_definitions(&self) -> Vec<StaffDefinition>;

    /// Résout le nom d'un skill à partir de son UID.
    fn resolve_skill_name(&self, skill_uid: &str) -> Option<String>;

    /// Retourne les ligues associées à un roster.
    fn find_leagues_for_roster(&self, roster_uid: &str) -> Vec<String>;

    /// Retourne les règles spéciales associées à un roster.
    fn find_special_rules_for_roster(&self, roster_uid: &str) -> Vec<String>;
}
```

### DTOs du port (dans `team_creation/ports.rs`)

```rust
pub struct RosterDefinition {
    pub uid: String,
    pub name: String,
    pub reroll_cost: u32,           // en gPo (pas kPo)
    pub available_players: Vec<PlayerPositionDefinition>,
    pub allowed_staff_uids: Vec<String>,
}

pub struct PlayerPositionDefinition {
    pub uid: String,
    pub position_name: String,
    pub cost: u32,                  // en gPo
    pub max_quantity: u8,
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub pa: u8,
    pub av: u8,
    pub skill_uids: Vec<String>,
}

pub struct StaffDefinition {
    pub uid: String,
    pub name: String,
    pub price: u32,
    pub max_quantity: u8,
}
```

### Implémentation (infrastructure)

```rust
// src/app/team_creation/io/reference_data_adapter.rs
// (ou src/infrastructure/reference_data_adapter.rs)

use crate::app::references::domain::port::IReferenceRepository;
use crate::app::team_creation::ports::IReferenceDataPort;

pub struct ReferenceDataAdapter {
    repo: Arc<dyn IReferenceRepository>,
}

impl IReferenceDataPort for ReferenceDataAdapter {
    fn find_roster_definition(&self, roster_uid: &str) -> Option<RosterDefinition> {
        // Traduit RefTeam → RosterDefinition
    }
    // ...
}
```

### Injection dans le contexte

```rust
// team_creation/context.rs
pub struct TeamCreationContext {
    pub team_repository: Arc<dyn ITeamDraftRepository>,
    pub roster_repository: Arc<dyn ITeamRosterRepository>,
    pub reference_data: Arc<dyn IReferenceDataPort>,  // ← NOUVEAU
    pub event_bus: EventBus,
}

// main.rs — assemblage
let ref_data_adapter = Arc::new(ReferenceDataAdapter::new(
    references_ctx.repository.clone(),
));
let team_creation_ctx = TeamCreationContext::new(pool, event_bus, ref_data_adapter);
```

---

## Situation finale

- **Aucun import de `crate::app::references`** dans `team_creation/io/web/` ni `team_creation/domain/`
- Tous les handlers de build-team utilisent `state.team_creation.reference_data` au lieu de `state.references.repository`
- `build_roster_from_ref()` est réécrit pour utiliser `IReferenceDataPort`
- Le port est injectable et testable (on peut fournir un mock en test unitaire)
- `check-arch` ne signale plus de violation Axe 3 pour le BC `team_creation`

---

## Checklist

- [ ] Définir les DTOs `RosterDefinition`, `PlayerPositionDefinition`, `StaffDefinition` dans `ports.rs`
- [ ] Définir le trait `IReferenceDataPort` dans `ports.rs`
- [ ] Implémenter `ReferenceDataAdapter` dans `team_creation/io/reference_data_adapter.rs`
- [ ] Ajouter `reference_data: Arc<dyn IReferenceDataPort>` dans `TeamCreationContext`
- [ ] Câbler l'adapter dans `main.rs` à l'initialisation du contexte
- [ ] Migrer `build_roster_from_ref()` pour utiliser le port
- [ ] Migrer `build_hired_rows()` pour utiliser le port (résolution skills/stats)
- [ ] Migrer `build_player_positions()` pour utiliser le port
- [ ] Supprimer tous les imports `references::*` de `build_team.rs`
- [ ] Supprimer les accès `state.references.repository` des handlers `build_team.rs`
- [ ] Vérifier `check-arch` — aucune violation Axe 3 pour `team_creation`
