# `Entity::eq()` fait concurrence à `PartialEq::eq`

**Priorité : moyenne**
**Dépend de** : —
**Fichier :** `src/app/shared_kernel/identity/ids.rs:15`
**Remplace :** la carte 09 (`09-entity-trait-eq`), écrite quand le trait vivait dans
`src/app/shared_kernel/common_types.rs`

## Problème

Le trait `Entity` définit une méthode `eq()` alors qu'il exige déjà `PartialEq` :

```rust
pub trait Entity: PartialEq<Self> {
    fn get_id(&self) -> EntityId;

    fn get_created_by(&self) -> EntityId;

    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}
```

Un type qui implémente `Entity` a donc deux méthodes `eq` : celle de `PartialEq` et
celle d'`Entity`. Selon le trait importé, `.eq()` peut ne pas appeler la même — c'est un
piège silencieux.

Les trois types qui implémentent `Entity` comparent déjà par identifiant dans leur
`PartialEq` :

- `DraftTeam` — `src/app/team_creation/domain/team_draft.rs`
- `RulesetSelectedTeam` — `src/app/team_creation/domain/team_ruleset_selected.rs`
- `RosterSelectedTeam` — `src/app/team_creation/domain/team_roster_selected.rs`

## Action

Supprimer la méthode `eq` du trait `Entity`. La comparaison par identifiant reste portée
par le `PartialEq` de chaque type, sans changement de comportement.

## Terminé quand

- `trait Entity` ne déclare plus que `get_id` et `get_created_by`.
- `make test`, `make lint` et `make arch` passent.
