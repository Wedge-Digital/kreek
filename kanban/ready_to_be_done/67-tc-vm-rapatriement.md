# BC `team_creation` — Rapatriement des View Models et builders

**Priorité : haute (bloquant pour 68–72)**
**Dépend de :** `66-tc-acl-reference-data-port.md`
**Contexte :** BC `team_creation` — couche IO web

## Objectif

Déplacer les view models et fonctions builder qui ne sont consommés que par `build_team.rs` depuis `references/io/web/pickers.rs` vers le BC `team_creation`. Ces VMs sont des types de la couche présentation du BC `team_creation`, pas du BC `references`.

---

## Situation actuelle

Les types suivants vivent dans `src/app/references/io/web/pickers.rs` mais ne sont utilisés que par `build_team.rs` :
- `HiredPlayerRowVm` — VM d'une ligne de joueur recruté
- `PlayerPositionVm` — VM d'un poste de joueur disponible
- `RosterPickerItemWithTier` — VM d'un roster avec info de tier
- `build_player_positions()` — builder qui prend un `RefTeam` + `IReferenceRepository`
- `build_roster_items_with_tiers()` — builder qui prend `IReferenceRepository` + `CreationRules`

Les types restants dans `pickers.rs` sont bien consommés par d'autres widgets du BC `references` :
- `RosterPickerItem` — utilisé par le roster picker widget `references`
- `InducementPickerItem` — utilisé par l'inducement picker
- `StarPlayerPickerItem` — utilisé par le star player picker

---

## Conception

### Nouveau fichier : `team_creation/io/web/view_models.rs`

Regrouper dans ce fichier :
- `HiredPlayerRowVm`
- `PlayerPositionVm`
- `RosterPickerItemVm` (renommé depuis `RosterPickerItemWithTier`)
- `StaffRowVm` (déjà dans `build_team.rs`, déplacer ici)
- `RerollVm` (idem)
- `CartLineVm`, `CartVm` (idem)

### Nouveau fichier : `team_creation/io/web/builders.rs`

Regrouper dans ce fichier :
- `build_hired_rows()` — réécrits pour utiliser `IReferenceDataPort` (cf. carte 66)
- `build_player_positions()` — réécrits pour utiliser `IReferenceDataPort`
- `build_roster_items_with_tiers()` — réécrits pour utiliser `IReferenceDataPort`
- `build_staff_rows()` (déjà dans `build_team.rs`)
- `build_cart_vm()` (déjà dans `build_team.rs`)
- `build_roster_from_ref()` → renommé `build_roster_from_definition()`, utilise `RosterDefinition` du port

### Nettoyage de `pickers.rs`

Supprimer de `references/io/web/pickers.rs` :
- `HiredPlayerRowVm`
- `PlayerPositionVm`
- `RosterPickerItemWithTier`
- `build_player_positions()`
- `build_roster_items_with_tiers()`

Conserver :
- `RosterPickerItem`, `build_roster_items()`
- `InducementPickerItem`
- `StarPlayerPickerItem`, `build_star_player_items()`

---

## Situation finale

- `build_team.rs` n'importe plus rien de `references/io/web/pickers`
- Tous les VMs de la page build-team sont dans `team_creation/io/web/view_models.rs`
- Tous les builders sont dans `team_creation/io/web/builders.rs`
- `build_team.rs` est allégé : il ne contient plus que les handlers et les structs de template
- Les builders utilisent `IReferenceDataPort` au lieu de `IReferenceRepository`
- `pickers.rs` ne contient plus que les types consommés par les widgets du BC `references`

---

## Checklist

- [ ] Créer `team_creation/io/web/view_models.rs` avec tous les VMs
- [ ] Créer `team_creation/io/web/builders.rs` avec tous les builders
- [ ] Réécrire `build_player_positions()` pour utiliser `IReferenceDataPort`
- [ ] Réécrire `build_roster_items_with_tiers()` pour utiliser `IReferenceDataPort`
- [ ] Réécrire `build_hired_rows()` pour utiliser `IReferenceDataPort`
- [ ] Réécrire `build_roster_from_ref()` → `build_roster_from_definition()`
- [ ] Déplacer `StaffRowVm`, `RerollVm`, `CartLineVm`, `CartVm` dans `view_models.rs`
- [ ] Déplacer `build_staff_rows()`, `build_cart_vm()` dans `builders.rs`
- [ ] Supprimer les types/builders migrés de `pickers.rs`
- [ ] Mettre à jour les imports dans `build_team.rs`
- [ ] Déclarer les modules dans `team_creation/io/web/mod.rs`
- [ ] `cargo check` — aucune erreur
