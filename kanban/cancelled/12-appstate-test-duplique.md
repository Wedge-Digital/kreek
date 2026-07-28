# Construction de `AppState` dupliquée dans les tests

> **Annulée le 2026-07-29 — sans objet.** La carte 245 (sous-états `FromRef`) a
> fait passer les handlers d'auth et de spaces sur leur propre contexte : le
> test de `login_submit` construit désormais un `AuthContext` seul, et celui de
> `coach_search` un `SpacesContext`. Vérification après coup :
> `grep -rn "AppState {" src/` ne remonte plus que `main.rs` (la construction
> réelle) et `state.rs` (la déclaration du type). **Aucun test du projet ne
> construit d'`AppState`** — la duplication que cette carte visait n'existe
> plus, et pas seulement pour les deux BCs extraits comme la 245 le prévoyait.
>
> Le helper de test proposé ci-dessous n'a donc plus de consommateur potentiel.
> Si un futur handler d'un autre BC redevient testable unitairement, c'est un
> sous-état `FromRef` qu'il faudra, pas un constructeur d'`AppState`.

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
