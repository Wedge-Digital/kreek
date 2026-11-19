# BC `team_creation` — Rapatriement VMs, builders + domain service roster

**Priorité : haute (bloquant pour 68–72)**
**Dépend de :** `66-tc-acl-reference-data-port.md`
**Contexte :** BC `team_creation` — use_cases + couche IO web

## Objectif

1. Extraire les view models et builders de `build_team.rs` vers des fichiers dédiés
2. Créer un **domain service** `roster_service` dans `use_cases/` qui encapsule la transformation `RosterDefinition` (DTO du port) → `Roster` (objet domaine). Les handlers n'accèdent jamais aux DTOs du port directement.
3. Nettoyer `pickers.rs` dans le BC `references`

---

## Situation actuelle (post carte 66)

`build_team.rs` contient tout en vrac :
- VMs : `StaffRowVm`, `RerollVm`, `CartLineVm`, `CartVm`, `HiredPlayerRowVm`, `PlayerPositionVm`, `RosterPickerItemWithTier`
- Builders de VMs : `build_hired_rows()`, `build_player_positions()`, `build_roster_items_with_tiers()`, `build_staff_rows()`, `build_reroll_vm()`, `build_cart_vm()`
- Mapping domaine : `build_roster_from_definition()` + `staff_kind()` — logique métier déguisée en code de présentation
- Les handlers manipulent directement `RosterDefinition` (DTO du port) pour construire des `Roster`

Dans `references/io/web/pickers.rs` : `HiredPlayerRowVm`, `PlayerPositionVm`, `RosterPickerItemWithTier`, `build_player_positions()`, `build_roster_items_with_tiers()` ne sont plus importés par personne (carte 66 les a recréés localement).

---

## Conception

### 1. Domain service : `use_cases/roster_service.rs`

Encapsule toute la logique de transformation port → domaine. Les handlers appellent ce service, jamais les DTOs du port.

```rust
// team_creation/use_cases/roster_service.rs

use crate::app::team_creation::ports::IReferenceDataPort;
use crate::app::team_creation::domain::roster::Roster;

/// Charge un Roster (objet domaine) à partir du référentiel externe.
pub fn load_roster(
    roster_uid: &str,
    ref_data: &dyn IReferenceDataPort,
) -> Option<Roster> {
    let def = ref_data.find_roster_definition(roster_uid)?;
    // Toute la logique de build_roster_from_definition() + staff_kind() vit ici
    Some(build_roster(def, ref_data))
}

/// Construit la liste des rosters disponibles pour un set de règles de création.
pub fn list_available_rosters(
    ref_data: &dyn IReferenceDataPort,
    rules: &CreationRules,
) -> Vec<RosterPickerItemWithTier> { ... }

/// Retourne les leagues/special rules associées à un roster.
pub fn roster_metadata(
    roster_uid: &str,
    ref_data: &dyn IReferenceDataPort,
) -> Option<RosterMetadata> { ... }
```

`RosterMetadata` (struct simple) porterait les `leagues` et `special_rules` nécessaires au handler `get_roster_players` pour l'auto-set.

### 2. Fichier VMs : `io/web/view_models.rs`

Regrouper :
- `HiredPlayerRowVm`
- `PlayerPositionVm`
- `RosterPickerItemWithTier`
- `StaffRowVm`, `RerollVm`
- `CartLineVm`, `CartVm`
- `RulesTierVm`, `RulesPanelVm`

### 3. Fichier builders VMs : `io/web/builders.rs`

Regrouper les fonctions qui transforment des objets **domaine** en VMs de présentation :
- `build_hired_rows(team: &RosterSelectedTeam, roster_def: &RosterDefinition) -> Vec<HiredPlayerRowVm>`
- `build_player_positions(roster_def: &RosterDefinition) -> Vec<PlayerPositionVm>`
- `build_staff_rows(team: &RosterSelectedTeam) -> Vec<StaffRowVm>`
- `build_reroll_vm(team: &RosterSelectedTeam) -> RerollVm`
- `build_cart_vm(team: &RosterSelectedTeam) -> CartVm`

Note : `build_hired_rows` et `build_player_positions` prennent encore un `&RosterDefinition` (DTO du port) car ils ont besoin des stats/skills pour les VMs. C'est acceptable ici car ce sont des builders de la couche **présentation** — ils transforment des données en VMs d'affichage. L'important est que les **handlers** ne construisent pas de `Roster` (objet domaine) à partir des DTOs du port eux-mêmes.

### 4. Nettoyage de `pickers.rs`

Supprimer de `references/io/web/pickers.rs` les types et builders orphelins :
- `HiredPlayerRowVm`, `PlayerPositionVm`, `RosterPickerItemWithTier`
- `build_player_positions()`, `build_roster_items_with_tiers()`

Conserver :
- `RosterPickerItem`, `build_roster_items()`
- `InducementPickerItem`
- `StarPlayerPickerItem`, `build_star_player_items()`

---

## Impact sur les handlers

### Avant (handler manipule les DTOs du port)

```rust
let ref_data = state.team_creation.reference_data.as_ref();
let roster_def = ref_data.find_roster_definition(&roster_uid).unwrap();
let roster = build_roster_from_definition(&roster_def, ref_data);
roster_team.choose_roster(roster)?;
```

### Après (handler appelle le domain service)

```rust
let ref_data = state.team_creation.reference_data.as_ref();
let roster = roster_service::load_roster(&roster_uid, ref_data)
    .ok_or(StatusCode::NOT_FOUND)?;
roster_team.choose_roster(roster)?;
```

---

## Structure fichiers — état final

```
src/app/team_creation/
├── use_cases/
│   ├── mod.rs
│   ├── roster_service.rs      ← NOUVEAU : port → domaine
│   ├── hire_player.rs
│   ├── fire_player.rs
│   └── ...
├── io/web/
│   ├── mod.rs
│   ├── build_team.rs          ← allégé : handlers + templates seulement
│   ├── view_models.rs         ← NOUVEAU : tous les VMs
│   ├── builders.rs            ← NOUVEAU : domaine → VMs
│   └── ...
└── ports.rs                   ← trait IReferenceDataPort + DTOs (inchangé)
```

---

## Situation finale

- `build_team.rs` ne contient que les handlers et les structs de template Askama
- Les handlers n'importent jamais `RosterDefinition` ni aucun DTO du port — ils passent par `roster_service`
- `build_roster_from_definition()` et `staff_kind()` vivent dans `roster_service.rs`
- Les VMs sont dans `view_models.rs`, les builders VMs dans `builders.rs`
- `pickers.rs` ne contient plus que les types consommés par les widgets du BC `references`
- `cargo check` — 0 erreur

---

## Checklist

- [ ] Créer `use_cases/roster_service.rs` avec `load_roster()`, `list_available_rosters()`, `roster_metadata()`
- [ ] Déplacer `build_roster_from_definition()` et `staff_kind()` dans `roster_service.rs`
- [ ] Créer `io/web/view_models.rs` avec tous les VMs
- [ ] Créer `io/web/builders.rs` avec les builders VMs (`build_hired_rows`, `build_player_positions`, `build_staff_rows`, `build_reroll_vm`, `build_cart_vm`)
- [ ] Migrer les handlers de `build_team.rs` pour utiliser `roster_service` au lieu de manipuler `RosterDefinition`
- [ ] Supprimer les types/builders orphelins de `references/io/web/pickers.rs`
- [ ] Déclarer les modules dans `use_cases/mod.rs` et `io/web/mod.rs`
- [ ] Mettre à jour les imports dans `build_team.rs`
- [ ] Tests unitaires du `roster_service` (load_roster avec un mock du port)
- [ ] `cargo check` — aucune erreur
