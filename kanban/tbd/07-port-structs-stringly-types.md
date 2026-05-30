# Port structs stringly-typés dans le domaine

**Priorité : moyenne**
**Fichiers :** `competition_repository_port.rs`, `season_repository_port.rs`

## Problème

Les structs de retour des ports (couche domaine) utilisent des `String` bruts là où le domaine dispose de types forts :

```rust
// competition_repository_port.rs
pub struct CompetitionBaseInfo {
    pub admin_ids:   Vec<String>,  // devrait être Vec<CoachId>
    pub admin_names: Vec<String>,  // devrait être Vec<CoachName>
}

// season_repository_port.rs
pub struct SeasonFull {
    pub season_id:      String,  // devrait être SeasonId
    pub competition_id: String,  // devrait être CompetitionId
    ...
}
```

C'est une fuite de la couche infrastructure dans le domaine : les types stringly-typés viennent de la DB, pas du domaine. Le port doit parler le langage du domaine.

## Action

Remplacer les `String` par les types domaine correspondants dans toutes les structs de retour des ports. L'implémentation du repository se charge de la conversion (`SUlid::try_new(&s).unwrap_or_...`).
