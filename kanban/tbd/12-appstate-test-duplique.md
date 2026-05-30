# Construction de `AppState` dupliquée dans les tests

**Priorité : faible**
**Fichiers :** `post_login.rs`, `coach_search.rs`, potentiellement d'autres

## Problème

La construction d'un `AppState` complet pour les tests est copy-collée entre les fichiers de test des handlers. Chaque test file reproduit la même boilerplate de 40+ lignes :

```rust
let state = AppState {
    auth: AuthContext { ... },
    spaces: SpacesContext { ... },
    competitions: CompetitionsContext { ... },
    news: NewsContext { ... },
    references: ReferencesContext::new(),
    email_service: Arc::new(ConsoleEmailService),
    host_domain: "localhost:8080".into(),
    bypass_auth: false,
    event_bus: bus.clone(),
    app_event_bus: new_bus(),
};
```

Quand `AppState` gagne un nouveau champ, il faut mettre à jour chaque copie.

## Action

Créer un helper de test dans un module partagé :

```rust
// src/app/test_helpers.rs (cfg(test))
pub fn test_state() -> AppState { ... }
pub fn test_state_with_users(repo: impl IUserRepository) -> AppState { ... }
```
