# `todo!()` dans un fake utilisé en test

**Priorité : haute**
**Fichier :** `src/app/auth/io/web/post_login.rs:186`

## Problème

`FakeSpaceRepository::join_spaces` retourne `todo!()`. Ce fake est instancié dans les tests du handler `post_login`. Si le chemin `join_spaces` est exercé, le test paniquera — ce qui est trompeur car le test échoue pour une mauvaise raison.

```rust
async fn join_spaces(&self, space_ids: &[SpaceId], coach_id: &CoachId) -> Result<(), SpaceRepositoryError> {
    todo!()
}
```

## Action

Remplacer par `Ok(())` ou `Err(...)` selon le comportement attendu dans ce contexte de test.
