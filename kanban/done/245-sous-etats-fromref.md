# Sous-états `FromRef` — `AuthContext` et `SpacesContext` autoportants

**Priorité : haute**
**Dépend de :** 242 (chapeau) — indépendante de 243/244
**Fichiers :** `src/state.rs`, `src/main.rs`, `src/app/auth/context.rs`,
`src/app/spaces/context.rs`, 4 handlers auth, 8 handlers spaces

## Problème

Tous les handlers d'auth et de spaces prennent `State<AppState>`, or `AppState`
(`src/state.rs`) contient **les dix contextes de BC** de l'application. Un
handler de login dépend, au niveau du type, de `match_report`, `ranking`,
`references` et de tout le reste.

Ce que ces handlers utilisent réellement :

- **auth** : `state.auth.*`, `state.event_bus`, `state.email_service`, `state.host_domain`
- **spaces** : `state.spaces.*`, `state.event_bus`

Le symptôme est spectaculaire et mesurable. Le test de `login_submit`
(`src/app/auth/io/web/post_login.rs:62-183`) doit construire un `AppState`
complet : il contient donc **une centaine de lignes de fakes pour `news`,
`competitions` et `spaces`** — trois BCs qui n'ont rien à voir avec un login,
avec des `impl` de `IGroupRepository`, `IMatchDayRepository`,
`ITiebreakCatalogPort`, `IArticleRepository`, `ICommentRepository`… Chaque
nouvelle méthode ajoutée à l'un de ces ports casse le test de login.

Deux dépendances applicatives sont par ailleurs mal placées : `email_service`
et `host_domain` vivent dans `AppState` alors qu'ils ne servent qu'à auth
(`forgot_password.rs:98,102`, `send_reset_password_email.rs`).

## Action

### 1. Rapatrier les dépendances d'auth dans son contexte

`AuthContext` gagne `email_service: Arc<dyn IEmailService>` et
`host_domain: String`. Ils disparaissent de `AppState` (injectés dans
`AuthContext::new()` depuis `main.rs`).

### 2. Passer les handlers sur des sous-états

Implémenter `FromRef<AppState>` pour `AuthContext` et `SpacesContext`, et
remplacer dans les handlers :

```rust
// avant
State(state): State<AppState>   // puis state.auth.user_repository
// après
State(ctx): State<AuthContext>  // puis ctx.user_repository
```

Le routeur reste un `Router<AppState>` : `FromRef` fait la projection, aucun
changement dans `main.rs` côté montage des routeurs.

Fichiers concernés — auth : `post_login.rs`, `post_register.rs`,
`forgot_password.rs`, `reset_password.rs`. Spaces : `register_space.rs`,
`all_spaces.rs`, `join_spaces.rs`, `members_widget.rs`,
`widget_tester_controller.rs`, `controllers/widgets/coach_select.rs`,
`coach_search.rs`, `coach_search_results.rs`.

### 3. Nettoyer les tests devenus inutiles

Une fois `login_submit` sur `State<AuthContext>`, tout le bloc de fakes
news/competitions/spaces de `post_login.rs` disparaît. Vérifier le même effet
dans les tests de `coach_search.rs`.

## Cartes impactées

- **Carte 12** (« Construction de `AppState` dupliquée dans les tests ») devient
  **sans objet** pour auth et spaces : il n'y a plus d'`AppState` à construire
  dans ces tests. Elle reste pertinente pour les autres BCs — à requalifier, pas
  à annuler.
- **Carte 10** (« `bypass_auth` dans `AppState` ») touche le même fichier et va
  dans le même sens : les deux peuvent être faites dans la même session.

## Checklist

- [ ] `email_service` et `host_domain` déplacés de `AppState` vers `AuthContext`
- [ ] `FromRef<AppState>` implémenté pour `AuthContext` et `SpacesContext`
- [ ] Les 12 handlers passés sur leur sous-état
- [ ] `grep -rn "AppState" src/app/auth src/app/spaces` ne remonte plus rien
- [ ] Fakes news/competitions/spaces supprimés de `post_login.rs`
- [ ] Carte 12 requalifiée (périmètre réduit aux BCs restants)
- [ ] `make check-arch` au vert, `make test` au vert
