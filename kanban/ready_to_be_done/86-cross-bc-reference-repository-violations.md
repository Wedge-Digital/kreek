# Violations cross-BC : accès direct à IReferenceRepository

**Priorité : moyenne**
**Fichiers concernés :**
- `src/app/teams/io/web/team_detail.rs`
- `src/app/players/context.rs`
- `src/app/players/io/web/player_table.rs`

## Problème

Les BCs `teams` et `players` importent directement `IReferenceRepository` du BC `references` au lieu de passer par un port/adapter dédié. C'est une violation de la règle de souveraineté des données entre BCs.

```rust
// teams/io/web/team_detail.rs
use crate::app::references::domain::port::IReferenceRepository;

// players/context.rs
use crate::app::references::domain::port::IReferenceRepository;

// players/io/web/player_table.rs
use crate::app::references::domain::port::IReferenceRepository;
```

Le pattern correct existe déjà dans le projet :
- `team_creation` → `references` via `IReferenceDataPort` + `ReferenceDataAdapter`
- `competitions` → `teams` via `ITeamInfoPort` + `TeamInfoAdapter`

## Action

### BC `teams`

1. Définir un port `IRosterInfoPort` dans `src/app/teams/ports.rs` avec les DTOs nécessaires (uniquement les données utilisées par `team_detail.rs`)
2. Créer l'adapter `src/infrastructure/teams/roster_info_adapter.rs` qui implémente ce port en appelant `IReferenceRepository`
3. Injecter le port dans `TeamsContext` via `main.rs`
4. Refactorer `team_detail.rs` pour utiliser le port au lieu de `IReferenceRepository`

### BC `players`

1. Définir un port `IPlayerReferencePort` dans `src/app/players/ports.rs` avec les DTOs nécessaires (données utilisées par `player_table.rs` pour résoudre les skills de base)
2. Créer l'adapter `src/infrastructure/players/player_reference_adapter.rs`
3. Injecter le port dans `PlayersContext` via `main.rs`
4. Refactorer `player_table.rs` et `context.rs` pour utiliser le port

## Hors scope

- `AuthSession` importé cross-BC : pattern Axum-login standard (extracteur middleware), non concerné par cette carte.

## Checklist

- [ ] Port `IRosterInfoPort` + DTOs dans `teams/ports.rs`
- [ ] Adapter `infrastructure/teams/roster_info_adapter.rs`
- [ ] Injection dans `TeamsContext` + `main.rs`
- [ ] Refacto `team_detail.rs` → utilise le port
- [ ] Port `IPlayerReferencePort` + DTOs dans `players/ports.rs`
- [ ] Adapter `infrastructure/players/player_reference_adapter.rs`
- [ ] Injection dans `PlayersContext` + `main.rs`
- [ ] Refacto `player_table.rs` + `context.rs` → utilise le port
- [ ] Suppression de tout import `crate::app::references` dans les BCs `teams` et `players`
- [ ] `cargo check` passe sans erreur
