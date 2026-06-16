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

### Interdiction des types primitifs nus — règle obligatoire (principe CQRS)

Règle issue des principes CQRS appliqués à ce projet : **tout ce qui entre dans le système doit être validé**, et **tout ce qui constitue le domaine suit la même exigence**. Seul ce qui **sort** (lecture/query) peut être un view model composé de types primitifs.

- Côté écriture (command) : commandes applicatives, agrégats, entités, événements domaine → **aucun type primitif nu**, toujours un value object (nutype) avec ses règles de validité.
- Côté lecture (query) : view models, DTOs de repository port retournés par des méthodes `find_*`/`list_*`/`search_*` → les primitives sont acceptées, car ces types ne portent aucune invariant à protéger, seulement des données à afficher.

Les types primitifs (`String`, `u32`, `u8`, `i32`, `bool`) sont donc **interdits** dans :
- les agrégats et entités domaine
- les commandes applicatives
- les événements domaine

Utiliser systématiquement des **value objects** (newtypes) pour bénéficier de la vérification du compilateur :

```rust
// INTERDIT
pub team_id:  String,
pub treasury: u32,
pub delta:    i32,

// OBLIGATOIRE
pub team_id:  TeamId,    // newtype wrapper
pub treasury: Kpo,       // newtype pour les montants en kPo
pub delta:    KpoDelta,  // newtype pour les deltas signés
```

Les newtypes doivent dériver `Serialize` / `Deserialize` quand ils apparaissent dans des events persistés :

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kpo(pub u32);
```

**Exceptions autorisées :**
- View models (structs Askama / couche présentation) — les primitives y sont acceptées
- DTOs de lecture (query) renvoyés par les repository ports — convention de ce projet : ces types vivent dans des fichiers `*_port.rs` / `*_repository_port.rs`
- Requêtes SQL (`sqlx::query!`) — les types sqlx ont leurs propres contraintes
- `reason: Option<String>` et autres champs de texte libre sans validation domaine

---

## App events vs Domain events — règle fondamentale

**Domain events** : produits par le domaine en réponse à une commande ou une action. Ils enregistrent ce qui s'est passé dans le domaine. Persistés dans l'event store (si le BC est event sourcé). Nommés en termes de faits domaine — jamais en termes de leur origine externe.

**App events** : franchissent les frontières de BCs via l'app event bus. Ils viennent de l'extérieur du domaine et sont traités **exclusivement dans la couche IO** (listeners).

```
App event bus ──► Listener (couche IO)
                      │
                      ▼
                  Use case applicatif
                      │
                      ▼
                  Méthode domaine  ──► DomainEvent
                                            │
                                            ▼
                                       Event store
```

**Règle de nommage des domain events** : le nom décrit ce qui s'est passé dans le domaine, pas d'où vient le déclencheur.

```rust
// INTERDIT — nom qui trahit l'origine externe
MatchPlayedReceived { ... }
PlayerValueChanged  { ... }

// OBLIGATOIRE — nom en termes domaine
PostMatchSequenceStarted { ... }
PlayerValueAdjusted      { ... }
```

Le domaine ne connaît pas les app events. Il expose des méthodes de commande qui retournent des domain events. C'est le listener (IO) qui décide quelle commande domaine appeler en réponse à quel app event.

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

## Conventions widgets HTMX — règles fondamentales

Un widget est un fragment HTML autonome exposé par un BC via un endpoint GET. Il encapsule son rendu, son comportement et son CSS. Il est chargé par une page hôte sans que celle-ci connaisse ses détails internes.

### Règle 1 — Pas de références croisées entre BCs

Un BC ne référence **jamais** directement la widget d'un autre BC. La page hôte peut composer plusieurs widgets de BCs différents, mais chaque BC n'importe que ses propres URLs de widgets.

```rust
// INTERDIT — BC teams référence une route du BC players
hx-get="{{ players_routes.roster_widget() }}"  // dans un template du BC teams

// CORRECT — la page hôte (neutre) compose les deux
// ou chaque BC expose son propre endpoint qui connaît ses propres routes
```

### Règle 2 — Communication par événements DOM sur `body`

Les widgets **ne s'appellent pas mutuellement**. Ils publient leurs actions via des événements DOM, les consommateurs s'abonnent indépendamment.

```js
// Publication (dans le widget, au clic / à la sélection)
htmx.trigger(document.body, 'coachSelected', { id: '...', name: '...' });

// Abonnement Alpine (dans la page hôte ou un autre widget)
@coach-selected.window="doSomething($event.detail)"

// Abonnement HTMX (déclenche une requête)
hx-trigger="coachSelected from:body"
```

**Format des événements** : payload `{ id, name }` pour les entités sélectionnées. Nommer les événements en `camelCase` côté JS — HTMX les convertit automatiquement en `kebab-case` pour `@event.window`.

### Règle 3 — Isolation HTMX (`hx-disinherit="*"`)

L'élément racine d'un widget pose `hx-disinherit="*"` pour bloquer **tout** héritage d'attributs HTMX venant de la page hôte (`hx-vals`, `hx-headers`, `hx-params`, etc.).

```html
<!-- Widget coach-search — racine isolée -->
<div class="coaches-search-panel" hx-disinherit="*">
    ...
</div>
```

Sans cela, les `hx-vals` ou `hx-include` de la page hôte s'injectent silencieusement dans les requêtes du widget.

### Règle 4 — Paramètres contextuels baked dans l'URL

Les paramètres contextuels reçus par le widget (ex. `space_id`) sont **baked dans l'URL `hx-get`** par Askama lors du rendu. Ne pas les récupérer via `hx-include` pointant vers le DOM parent.

```html
<!-- CORRECT — space_id fourni par le serveur au rendu du widget -->
hx-get="{{ routes.spaces.coach_search_results() }}?space_id={{ space_id }}"
hx-params="q"

<!-- INTERDIT — couplage au DOM de la page hôte -->
hx-include="[name='space_id']"
```

### Règle 5 — CSS embarqué, pas de dépendance au layout

Chaque widget embarque son propre `<link rel="stylesheet">`. Il n'assume pas que la page hôte charge ses styles.

```html
<link rel="stylesheet" href="/static/css/widgets/coach-search.css">
<div class="coaches-search-panel" hx-disinherit="*">…</div>
```

### Règle 6 — Scripts sans ID globaux

Les scripts de comportement d'un widget (navigation clavier, init de composant tiers, etc.) référencent leur conteneur via `document.currentScript.previousElementSibling`, pas via un `id` global.

```html
<div class="coaches-search-panel" hx-disinherit="*">…</div>
<script>
(function () {
    const panel = document.currentScript.previousElementSibling;
    // tout le comportement est scoped à `panel`
})();
</script>
```

**Pourquoi :** évite les collisions si le même widget est présent plusieurs fois dans la page, et évite de polluer le namespace global.

### Règle 7 — Lifecycle des composants tiers (TomSelect, etc.)

Les composants JS tiers intégrés dans un widget sont wrappés dans un `x-data` Alpine avec `init()` et `destroy()` pour un cycle de vie propre lors des swaps HTMX.

```html
<div x-data="{
    init() { this._ts = new TomSelect(this.$refs.select, { ... }); },
    destroy() { this._ts?.destroy(); }
}">
    <select x-ref="select">…</select>
</div>
```

Ne jamais initialiser TomSelect (ou équivalent) dans un `<script>` nu sans lifecycle — le composant survivrait au remplacement du DOM et causerait des doublons.

---

## Conventions templates (Askama + HTMX)

- Un template de **page complète** pour le premier chargement
- Des templates de **fragments** pour les réponses HTMX (swap partiel)
- Les structs de template ne portent que des **view models** — pas d'entités domaine

### Interdiction des styles inline — règle obligatoire

Les attributs `style="..."` sont **totalement interdits** dans les templates HTML.

```html
<!-- INTERDIT -->
<div style="color: red; margin-top: 8px;">…</div>

<!-- OBLIGATOIRE — utiliser des classes CSS -->
<div class="text-error mt-2">…</div>
```

Tout besoin de style passe par des classes CSS définies dans les fichiers `.css` du projet (`assets/static/css/`).

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

### Couverture obligatoire — règle fondamentale

Toute fonctionnalité livrée doit être couverte par :
1. **Un test unitaire** (`cargo test`) — logique domaine/use case, co-localisé avec le code.
2. **Un test end-to-end** (`tests/e2e/`, pytest + Playwright) — comportement réel dans un navigateur contre le serveur dev lancé.

Le test unitaire vérifie la logique ; le test E2E vérifie que le rendu HTML/HTMX/Alpine.js produit fonctionne réellement (le bug du widget coach-search et celui des pickers de tiers en phase 2 n'auraient été détectés par aucun test unitaire — uniquement par un test E2E piloté en navigateur).

Voir `tests/e2e/README.md` pour l'exécution (`make e2e`, nécessite le serveur dev lancé).

---

## Pièges frontend connus — Alpine.js + HTMX

### Alpine.js : chargement unique dans le layout de base

Alpine CDN est chargé **uniquement dans `app-layout.html`** (`<head>`, avec `defer`).  
Ne jamais l'inclure dans un `{% block content %}` de page individuelle.

**Pourquoi :** HTMX navigue sans rechargement complet — il ré-exécute les `<script>` trouvés dans le contenu swappé. Si deux pages chargent chacune le CDN Alpine, la navigation entre elles via HTMX crée une **deuxième instance Alpine** en mémoire. Les deux instances se disputent l'initialisation des composants `x-data`, notamment les fragments injectés dynamiquement via `htmx.ajax`. Symptôme typique : le composant fonctionne sur rechargement complet de la page (F5) mais pas lors de la navigation HTMX.

Les fonctions Alpine (`finalizePage`, `skillPicker`, etc.) restent dans des `<script>` inline des pages — HTMX les ré-exécute correctement à chaque navigation.

### Fragments HTMX : ne pas répéter l'`id` du conteneur cible

Quand un fragment est injecté via `htmx.ajax` avec `swap: 'innerHTML'`, l'élément racine du fragment **ne doit pas avoir le même `id` que son conteneur**.

```html
<!-- INTERDIT — id dupliqué dans le DOM après injection -->
<!-- Conteneur dans la page : -->
<div id="skill-picker-container" x-show="selectedPlayerId"></div>
<!-- Fragment retourné par le serveur : -->
<div id="skill-picker-container" x-data="skillPicker(...)">...</div>

<!-- CORRECT — le fragment est le contenu du conteneur, pas le conteneur lui-même -->
<div x-data="skillPicker(...)">...</div>
```