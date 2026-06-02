# `Entity::eq()` shadowe `PartialEq::eq`

**Priorité : moyenne**
**Fichier :** `src/app/shared_kernel/common_types.rs:28`

## Problème

Le trait `Entity` définit une méthode `eq()` sur un type qui implémente déjà `PartialEq` :

```rust
pub trait Entity: PartialEq<Self> {
    fn get_id(&self) -> EntityId;
    fn get_created_by(&self) -> EntityId;

    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}
```

Un type implémentant `Entity` a deux méthodes `eq` : celle de `PartialEq` (comparaison structurelle) et celle de `Entity` (comparaison par ID). Selon le trait depuis lequel on appelle `.eq()`, le résultat peut différer — c'est un footgun silencieux.

## Action

Supprimer la méthode `eq` du trait `Entity` — c'est exactement ce que `PartialEq` est censé faire. Si la comparaison par ID est le comportement voulu, implémenter `PartialEq` en conséquence directement sur les types.
