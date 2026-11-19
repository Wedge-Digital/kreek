# `todo!()` en code de production

**Priorité : haute**
**Fichier :** `src/app/team_creation/domain/team_ruleset_selected.rs:18`

## Problème

Un `todo!()` existe dans du code de production. Si cette branche est atteinte à l'exécution, le processus paniquera sans aucun message d'erreur utile pour l'utilisateur.

```rust
// team_ruleset_selected.rs:18
fn some_method(&self) {
    todo!()
}
```

## Action

Soit implémenter la logique manquante, soit supprimer le code mort si cette branche n'est plus atteignable.
