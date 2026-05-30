# `bypass_auth` dans `AppState`

**Priorité : moyenne**
**Fichiers :** `src/state.rs`, `src/main.rs`, `src/web/middleware/bypass_auth.rs`

## Problème

Un flag de contournement d'authentification est propagé dans tout l'état applicatif :

```rust
pub struct AppState {
    ...
    pub bypass_auth: bool,
}
```

Ce flag est lu dans `AuthBackend` et `bypass_auth_middleware`. Il est accessible depuis n'importe quel handler via `State<AppState>`. En production, si ce flag est activé par erreur de configuration, l'authentification est désactivée globalement — sans aucune protection de compilation.

## Action

Compiler conditionnellement ce mécanisme :

```rust
#[cfg(debug_assertions)]
pub bypass_auth: bool,
```

Ou, mieux, supprimer le flag de `AppState` et le lire uniquement dans `main.rs` au moment de construire les layers, sans le propager dans l'état partagé.
