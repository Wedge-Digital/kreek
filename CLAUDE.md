# CLAUDE.md — kreek

Directives de travail pour Claude Code sur ce projet.

---

## Projet

Application web Rust avec backend Axum et frontend HTMX. Architecture orientée domaine, rendu HTML côté serveur via Askama.

---

## Stack technique cible

| Rôle | Crate | Version |
|---|---|---|
| HTTP framework | `axum` | 0.8 |
| Runtime async | `tokio` | 1 (features = ["full"]) |
| Auth + sessions | `axum-login` | 0.16 |
| Session middleware | `tower-sessions` | 0.13 |
| Templates HTML | `askama` | 0.12 |
| Base de données | `sqlx` | 0.8 (features = ["postgres", "runtime-tokio-native-tls", "macros", "time"]) |
| Sérialisation | `serde` | 1 (features = ["derive"]) |
| Erreurs domaine | `thiserror` | 1 |
| Config env | `config` | 0.14 |
| Dotenv local | `dotenvy` | 0.15 |
| Logging HTTP | `tower-http` | 0.6 (features = ["trace"]) |
| Tracing | `tracing-subscriber` | 0.3 |
| Hash passwords | `argon2` | 0.5 |
| IDs | `ulid` | 1 (type `Sulid` = wrapper local) |

---

## Structure cible

```
src/
├── main.rs                  # point d'entrée, composition des dépendances
├── config.rs                # AppConfig + load_config()
├── error.rs                 # AppError + IntoResponse
├── state.rs                 # AppState
├── services/                # services partagés (IdService, …)
│
├── domain/                  # pur — aucune dépendance framework
│   ├── mod.rs
│   ├── model/               # entités, value objects, agrégats
│   ├── ports/               # traits Repository + Service
│   └── error.rs             # DomainError (thiserror)
│
├── application/             # cas d'usage, orchestration
│   ├── mod.rs
│   └── commands/            # structs de commandes
│
├── infrastructure/
│   ├── mod.rs
│   ├── db/                  # implémentations sqlx des ports
│   └── auth.rs              # AuthBackend (axum-login)
│
├── web/
│   ├── mod.rs               # build_router()
│   ├── middleware/          # csrf, …
│   ├── handlers/            # handlers Axum par domaine
│   └── templates/           # structs Askama
│
└── templates/               # fichiers .html Askama
    ├── base.html
    ├── auth/
    └── [domaine]/
```

L'organisation actuelle (`src/app/<feature>/`) sera migrée vers cette structure au fur et à mesure.

---

## Injection de dépendances

Manuelle et explicite — pas de conteneur IoC.

- `Arc<dyn Trait + Send + Sync>` pour tout service ou repository partagé
- `PgPool` est déjà `Clone + Send + Sync` — pas d'`Arc` supplémentaire
- `Arc<AppConfig>` pour la configuration
- Ne jamais passer `&dyn Trait` nu dans `AppState`

---

## Middleware — ordre d'exécution

```
Request → TraceLayer → SessionLayer → AuthLayer → CsrfMiddleware → login_required! → Handler
```

Le middleware CSRF rejette les POST/PUT/DELETE/PATCH sans header `HX-Request: true`, sauf `/login` et `/logout`.

---

## Conventions handlers

- Un handler = une responsabilité
- Signature de retour : `Result<impl IntoResponse, AppError>`
- Aucune logique métier dans un handler — déléguer au service applicatif
- Utilisateur courant via `AuthSession` injecté par axum-login

---

## Conventions domaine

- `domain/` n'importe jamais de crate framework (axum, sqlx, tower, …)
- Value Objects : constructeur privé + smart constructor `new() -> Result<Self, DomainError>`
- Agrégats : n'exposent pas de référence mutable vers leur état interne
- `DomainError` : enum exhaustif avec `thiserror`

---

## Projections event sourcing — règle fondamentale

Toute mise à jour d'une table de projection doit s'exécuter **dans la même transaction base de données** que l'append de l'événement qui la déclenche.

```rust
// CORRECT — atomique
let mut tx = pool.begin().await?;
insert_event(&mut tx, event).await?;
update_projection(&mut tx, event).await?;
tx.commit().await?;

// INTERDIT — deux transactions séparées
insert_event(&pool, event).await?;          // si ça passe…
update_projection(&pool, event).await?;     // …et ça échoue : projection désynchronisée
```

Conséquences :
- Si la transaction échoue, ni l'événement ni la projection ne sont écrits — cohérence garantie sans coordination distribuée
- La projection est un **dérivé rebuildable** : en cas de désynchronisation exceptionnelle, on peut la reconstruire intégralement en rejouant l'event store
- `update_projection_in_tx()` reçoit toujours un `&mut PgConnection` (ou `&mut Transaction`), jamais un `&PgPool`

---

## Souveraineté des données entre BCs — règle fondamentale

Chaque BC est **souverain sur ses données** : il est formellement interdit à un BC d'effectuer des requêtes SQL sur des tables appartenant à un autre BC.

L'assemblage de données issues de plusieurs BCs se fait **exclusivement au niveau du frontend**, par composition de widgets HTMX. Chaque BC expose ses propres fragments HTML, chargés indépendamment par la page hôte :

```html
<!-- Page fournie par BC teams — il ignore tout des données joueurs -->
<div hx-get="{{ players_routes.team_roster_widget(space_id, team_id) }}"
     hx-trigger="load"
     hx-target="this">
</div>
<!-- Ce fragment est rendu et possédé par le BC players -->
```

Ce principe est déjà appliqué dans la page de construction d'équipe, où le widget de sélection du roster est fourni par le BC `references`.

Conséquences :
- Pas de projection locale de données d'un autre BC
- Pas de synchronisation de données entre BCs via des listeners sauf pour les **transitions d'état métier** (ex. : `TeamCreated` déclenche la création d'un agrégat dans `teams`)
- Aucun handler ne combine des requêtes SQL de deux BCs différents

---

## Conventions templates (Askama + HTMX)

- Un template de **page complète** pour le premier chargement
- Des templates de **fragments** pour les réponses HTMX (swap partiel)
- Les structs de template ne portent que des **view models** — pas d'entités domaine

### Réponses HTMX spéciales

```rust
// Redirect
Response::builder().header("HX-Redirect", "/dashboard").body(Body::empty()).unwrap()

// Refresh
Response::builder().header("HX-Refresh", "true").body(Body::empty()).unwrap()

// Trigger événement client
Response::builder().header("HX-Trigger", r#"{"showToast": "Sauvegardé"}"#).body(Body::empty()).unwrap()
```

---

## Gestion des erreurs

`AppError` est l'enum central (`src/error.rs`) qui implémente `IntoResponse`.  
Il convertit automatiquement `sqlx::Error` et `DomainError` via `#[from]`.  
Les handlers HTMX reçoivent un fragment HTML d'erreur, pas du JSON.

---

## Configuration

Variables d'environnement au format `APP__<SECTION>__<CLÉ>` (double underscore comme séparateur).

```bash
APP__DATABASE__URL=postgres://user:pass@localhost/kreek_dev
APP__AUTH__SESSION_SECRET=<min_32_chars>
APP__SERVER__PORT=3000
```

---

## Base de données

- Migrations dans `migrations/` avec `sqlx migrate`
- Requêtes SQL dans des fichiers `.sql` dédiés sous `repositories/sql/`
- Utiliser `sqlx::query_as!` (macro vérifiée à la compilation) de préférence à `query_as`
- Tests d'intégration sur une vraie base de données — pas de mock sqlx

---

## Sessions

Phase 1 : `MemoryStore` (implémenté dans `main.rs`).  
Phase 2 : migration vers `RedisStore` — le changement est localisé à `main.rs` uniquement.

---

## Tests

- Tests unitaires dans un module `tests/` co-localisé avec le code
- Tests d'intégration repository : utilisent une vraie PgPool
- Fixtures SQL dans `tests/fixtures/*.sql`
- Ne pas mocker sqlx — les tests doivent frapper une vraie base