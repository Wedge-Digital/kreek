# Fonction `initials()` dupliquée

**Priorité : faible**
**Fichiers :** `competition_detail.rs:67`, `coach_search.rs:67`

## Problème

La même fonction privée est définie deux fois, à l'identique :

```rust
fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}
```

Toute modification (ex : gestion des tirets, accents) devra être faite deux fois — et il est facile d'en oublier une.

## Action

Déplacer dans `src/app/shared_kernel/` en tant que fonction publique (ou méthode sur `CoachName`) et importer depuis les deux modules.
