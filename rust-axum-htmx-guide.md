# Guide d'implémentation — Backend Rust / Axum / HTMX

> Guide de référence pour Claude Code. Décrit l'architecture cible, les choix techniques, les patterns et les conventions à respecter lors de l'implémentation.

---

## Stack technique

| Rôle | Crate | Version |
|---|---|---|
| HTTP framework | `axum` | 0.7 |
| Runtime async | `tokio` | 1 (features = ["full"]) |
| Auth + sessions | `axum-login` | 0.16 |
| Session middleware | `tower-sessions` | 0.13 |
| Templates HTML | `askama` | 0.12 |
| Base de données | `sqlx` | 0.7 (features = ["postgres", "uuid"]) |
| Sérialisation | `serde` | 1 (features = ["derive"]) |
| Erreurs domaine | `thiserror` | 1 |
| Config env | `config` | 0.14 |
| Dotenv local | `dotenvy` | 0.15 |
| Logging HTTP | `tower-http` | 0.5 (features = ["trace"]) |
| Tracing | `tracing-subscriber` | 0.3 |
| Hash passwords | `argon2` | 0.5 |

```toml
[dependencies]
axum               = "0.7"
tokio              = { version = "1", features = ["full"] }
axum-login         = "0.16"
tower-sessions     = "0.13"
askama             = "0.12"
sqlx               = { version = "0.7", features = ["postgres", "uuid"] }
serde              = { version = "1", features = ["derive"] }
thiserror          = "1"
dotenvy            = "0.15"
config             = "0.14"
tower-http         = { version = "0.5", features = ["trace"] }
tracing-subscriber = "0.3"
argon2             = "0.5"
```

---

## Structure du projet

```
src/
├── main.rs                  # point d'entrée, composition des dépendances
├── config.rs                # struct AppConfig + loader
├── error.rs                 # AppError + IntoResponse
├── state.rs                 # AppState
│
├── domain/                  # pur, sans dépendances framework
│   ├── mod.rs
│   ├── model/               # entités, value objects, agrégats
│   ├── ports/               # traits Repository, traits Service
│   └── error.rs             # DomainError
│
├── application/             # cas d'usage, orchestration
│   ├── mod.rs
│   └── commands/            # structs de commandes
│
├── infrastructure/
│   ├── mod.rs
│   ├── db/                  # implémentations sqlx des repositories
│   └── auth.rs              # AuthBackend
│
├── web/
│   ├── mod.rs               # build_router()
│   ├── middleware/          # csrf, logging custom
│   ├── handlers/            # handlers Axum par domaine
│   └── templates/           # structs Askama
│
└── templates/               # fichiers .html Askama
    ├── base.html
    ├── auth/
    └── [domaine]/
```

---

## Configuration

### Struct typée

```rust
// src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server:   ServerConfig,
    pub database: DatabaseConfig,
    pub auth:     AuthConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url:             String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub session_secret:           String,
    pub session_duration_hours:   u64,
}

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    config::Config::builder()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 3000)?
        .set_default("database.max_connections", 10)?
        .set_default("auth.session_duration_hours", 8)?
        .add_source(config::File::with_name("config/default").required(false))
        .add_source(
            config::File::with_name(&format!(
                "config/{}",
                std::env::var("APP_ENV").unwrap_or("development".into())
            ))
            .required(false),
        )
        .add_source(
            config::Environment::with_prefix("APP")
                .separator("__")
                .try_parsing(true),
        )
        .build()?
        .try_deserialize()
}
```

### Variables d'environnement

Convention : `APP__<SECTION>__<CLÉ>` avec double underscore comme séparateur de niveau.

```bash
# .env (dev uniquement — ne pas commiter)
APP__DATABASE__URL=postgres://user:pass@localhost/myapp_dev
APP__AUTH__SESSION_SECRET=dev_secret_min_32_chars_change_in_prod
APP__SERVER__PORT=3000
APP__DATABASE__MAX_CONNECTIONS=10
```

```bash
# Production — injectées par Docker / K8s / systemd
APP__DATABASE__URL=postgres://user:pass@prod-db/myapp
APP__AUTH__SESSION_SECRET=<secret_long_aléatoire>
APP__SERVER__PORT=3000
```

---

## AppState — injection de dépendances

L'injection de dépendances est **manuelle et explicite**. Pas de conteneur IoC, pas de réflexion. Toutes les dépendances sont construites dans `main.rs` et passées via `AppState`.

```rust
// src/state.rs
use std::sync::Arc;
use sqlx::PgPool;
use crate::config::AppConfig;
use crate::domain::ports::{OrderRepository, OrderService};

#[derive(Clone)]
pub struct AppState {
    pub config:        Arc<AppConfig>,
    pub db_pool:       PgPool,
    pub order_service: Arc<dyn OrderService + Send + Sync>,
    // ajouter les services par domaine ici
}
```

### Règles d'injection

- `Arc<dyn Trait + Send + Sync>` pour tout service ou repository partagé entre handlers
- `PgPool` est déjà `Clone + Send + Sync` — pas besoin d'`Arc` supplémentaire
- `Arc<AppConfig>` pour la configuration
- **Ne jamais** passer une référence nue `&dyn Trait` dans `AppState` — utiliser `Arc`

---

## main.rs

```rust
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_sessions::{MemoryStore, SessionManagerLayer, Expiry};
use tower_sessions::cookie::{time::Duration, SameSite};
use axum_login::AuthManagerLayerBuilder;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = crate::config::load_config().unwrap_or_else(|e| {
        eprintln!("❌ Configuration invalide : {}", e);
        std::process::exit(1);
    });

    let db_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await
        .expect("Connexion DB impossible");

    // Sessions en mémoire (phase 1 — remplacer par RedisStore pour HA)
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)            // passer à true en prod derrière HTTPS
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(
            Duration::hours(config.auth.session_duration_hours as i64)
        ));

    // Construction des dépendances (racine de composition)
    let order_repo    = Arc::new(infrastructure::db::PostgresOrderRepository::new(db_pool.clone()));
    let order_service = Arc::new(application::OrderServiceImpl::new(order_repo.clone()));

    let state = AppState {
        config:        Arc::new(config.clone()),
        db_pool:       db_pool.clone(),
        order_service,
    };

    let auth_backend = infrastructure::auth::AuthBackend::new(db_pool);
    let auth_layer   = AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("🚀 Démarrage sur http://{}", addr);

    axum::serve(
        TcpListener::bind(&addr).await.unwrap(),
        web::build_router(state, auth_layer),
    )
    .await
    .unwrap();
}
```

---

## Router et middleware

```rust
// src/web/mod.rs
use axum::{Router, middleware};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use axum_login::AuthManagerLayer;

pub fn build_router(state: AppState, auth_layer: AuthManagerLayer<...>) -> Router {
    Router::new()
        // Routes protégées
        .nest("/orders",  handlers::order::router())
        .nest("/catalog", handlers::catalog::router())
        .route_layer(login_required!(AuthBackend, login_url = "/login"))
        // Routes publiques
        .route("/login",  get(handlers::auth::login_form).post(handlers::auth::login))
        .route("/logout", post(handlers::auth::logout))
        // Middleware stack — ordre d'exécution : top → bottom
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::from_fn(csrf_middleware))
                .layer(auth_layer),
        )
        .with_state(state)
}
```

### Chaîne d'exécution des middleware

```
Request entrante
      │
      ▼
 TraceLayer          ← log method, path, status, latence
      │
      ▼
 SessionLayer        ← charge la session depuis MemoryStore
      │
      ▼
 AuthLayer           ← peuple AuthSession<AuthBackend>
      │
      ▼
 CsrfMiddleware      ← vérifie HX-Request sur POST/PUT/DELETE/PATCH
      │
      ▼
 login_required!     ← redirige vers /login si non authentifié
      │
      ▼
 Handler             ← reçoit State<AppState> + AuthSession garantis
```

---

## Middleware CSRF

Vérifie que les requêtes mutantes viennent bien de HTMX (header `HX-Request: true`).

```rust
// src/web/middleware/csrf.rs
use axum::{extract::Request, middleware::Next, response::Response, http::Method};
use crate::error::AppError;

pub async fn csrf_middleware(request: Request, next: Next) -> Result<Response, AppError> {
    let is_mutating = matches!(
        request.method(),
        &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH
    );

    if is_mutating {
        let is_htmx = request
            .headers()
            .get("HX-Request")
            .map_or(false, |v| v == "true");

        // Exclure les routes de login (formulaire HTML classique)
        let path = request.uri().path().to_string();
        let is_auth_route = path.starts_with("/login") || path.starts_with("/logout");

        if !is_htmx && !is_auth_route {
            return Err(AppError::Forbidden);
        }
    }

    Ok(next.run(request).await)
}
```

---

## Authentification

### User + AuthBackend

```rust
// src/io/auth.rs
use axum_login::{AuthUser, AuthnBackend, UserId};
use argon2::{Argon2, PasswordHash, PasswordVerifier};

#[derive(Clone, sqlx::FromRow)]
pub struct User {
    pub id:            uuid::Uuid,
    pub email:         String,
    pub password_hash: String,
}

impl AuthUser for User {
    type Id = uuid::Uuid;

    fn id(&self) -> Self::Id { self.id }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

#[derive(Deserialize, Clone)]
pub struct Credentials {
    pub email:    String,
    pub password: String,
}

#[derive(Clone)]
pub struct AuthBackend {
    pool: sqlx::PgPool,
}

impl AuthBackend {
    pub fn new(pool: sqlx::PgPool) -> Self { Self { pool } }
}

#[async_trait::async_trait]
impl AuthnBackend for AuthBackend {
    type User        = User;
    type Credentials = Credentials;
    type Error       = crate::error::AppError;

    async fn authenticate(&self, creds: Credentials) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash FROM users WHERE email = $1",
            creds.email
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(user) = user else { return Ok(None) };

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| AppError::InternalError)?;

        let is_valid = Argon2::default()
            .verify_password(creds.password.as_bytes(), &parsed_hash)
            .is_ok();

        Ok(if is_valid { Some(user) } else { None })
    }

    async fn get_user(&self, id: &UserId<Self>) -> Result<Option<User>, AppError> {
        sqlx::query_as!(User, "SELECT id, email, password_hash FROM users WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::from)
    }
}
```

### Handlers auth

```rust
// src/web/handlers/auth.rs
type AuthSession = axum_login::AuthSession<AuthBackend>;

pub async fn login_form(auth_session: AuthSession) -> impl IntoResponse {
    if auth_session.user.is_some() {
        return Redirect::to("/dashboard").into_response();
    }
    Html(LoginTemplate {}.render().unwrap()).into_response()
}

pub async fn login(
    mut auth_session: AuthSession,
    Form(creds): Form<Credentials>,
) -> impl IntoResponse {
    match auth_session.authenticate(creds).await {
        Ok(Some(user)) => {
            auth_session.login(&user).await.unwrap();
            Response::builder()
                .header("HX-Redirect", "/dashboard")
                .body(Body::empty())
                .unwrap()
        }
        Ok(None) => Html(
            r#"<div id="login-error" class="error">Email ou mot de passe incorrect</div>"#
        ).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
    auth_session.logout().await.unwrap();
    Redirect::to("/login")
}
```

---

## Templates HTMX avec Askama

Les templates sont **compilés et typés** à la compilation. Une variable manquante ou mal typée est une erreur de compilation.

### Structure

```
templates/
├── base.html              # layout principal avec hx-boost
├── auth/
│   └── login.html
└── orders/
    ├── list.html          # page complète
    ├── row.html           # fragment — retourné par les handlers HTMX
    └── form.html          # fragment formulaire
```

### Template de fragment HTMX

```rust
// src/web/templates/orders.rs
use askama::Template;

#[derive(Template)]
#[template(path = "orders/row.html")]
pub struct OrderRowTemplate {
    pub order: OrderView,
}

// Handler qui retourne un fragment
pub async fn confirm_order(
    Path(id): Path<Uuid>,
    AuthSession { user, .. }: AuthSession,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let order = state.order_service.confirm(id, user.id).await?;
    let html  = OrderRowTemplate { order }.render()?;
    Ok(Html(html))
}
```

```html
<!-- templates/orders/row.html -->
<tr id="order-{{ order.id }}" hx-target="this" hx-swap="outerHTML">
  <td>{{ order.reference }}</td>
  <td>{{ order.status }}</td>
  <td>
    {% if order.is_confirmable %}
    <button hx-post="/orders/{{ order.id }}/confirm"
            hx-confirm="Confirmer cette commande ?">
      Confirmer
    </button>
    {% endif %}
  </td>
</tr>
```

---

## Gestion des erreurs

### AppError centralisé

```rust
// src/error.rs
use axum::{response::{IntoResponse, Response}, http::StatusCode};
use askama::Template;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Non authentifié")]
    Unauthorized,

    #[error("Accès interdit")]
    Forbidden,

    #[error("Ressource introuvable")]
    NotFound,

    #[error("Erreur domaine : {0}")]
    Domain(#[from] crate::domain::error::DomainError),

    #[error("Erreur base de données : {0}")]
    Database(#[from] sqlx::Error),

    #[error("Erreur interne")]
    InternalError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized  => (StatusCode::UNAUTHORIZED,   self.to_string()),
            AppError::Forbidden     => (StatusCode::FORBIDDEN,       self.to_string()),
            AppError::NotFound      => (StatusCode::NOT_FOUND,       self.to_string()),
            AppError::Domain(e)     => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
            _                       => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne".into()),
        };

        // Retourne un fragment HTMX pour les requêtes HTMX, JSON sinon
        // (adapter selon les besoins)
        let body = format!(r#"<div class="error-toast">{}</div>"#, message);
        (status, Html(body)).into_response()
    }
}
```

---

## Domain — Value Objects et Agrégats

### Value Object — smart constructor

```rust
// src/domain/model/email.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);  // champ privé — construction contrôlée

impl Email {
    pub fn new(value: String) -> Result<Self, DomainError> {
        if !value.contains('@') {
            return Err(DomainError::InvalidEmail(value));
        }
        Ok(Email(value))
    }

    pub fn value(&self) -> &str { &self.0 }
}
```

### Agrégat avec CQRS/ES

```rust
// src/domain/model/order.rs
#[derive(Debug, Clone)]
pub struct Order {
    pub id:                OrderId,
    pub status:            OrderStatus,
    pub lines:             Vec<OrderLine>,
    uncommitted_events:    Vec<OrderEvent>,  // privé
}

impl Order {
    // Méthode de commande : validation + event + apply
    pub fn confirm(&mut self) -> Result<(), DomainError> {
        if self.status != OrderStatus::Pending {
            return Err(DomainError::InvalidOrderStatus);
        }
        let event = OrderEvent::Confirmed { at: chrono::Utc::now() };
        self.apply(&event);
        self.uncommitted_events.push(event);
        Ok(())
    }

    // Apply : mutation pure, idempotente, rejouée à la reconstruction
    fn apply(&mut self, event: &OrderEvent) {
        match event {
            OrderEvent::Confirmed { at } => {
                self.status = OrderStatus::Confirmed;
            }
            // exhaustivité garantie à la compilation
        }
    }

    pub fn take_uncommitted_events(&mut self) -> Vec<OrderEvent> {
        std::mem::take(&mut self.uncommitted_events)
    }
}
```

---

## Migration vers Redis (phase 2)

Quand le besoin de haute disponibilité ou de révocation immédiate de sessions se présente, le changement est localisé à `main.rs` :

```rust
// Ajouter dans Cargo.toml
// tower-sessions-redis-store = "0.14"

// Remplacer dans main.rs :

// Avant (phase 1)
let session_store = MemoryStore::default();

// Après (phase 2)
use tower_sessions_redis_store::{RedisStore, fred::prelude::*};

let redis_config = RedisConfig::from_url(&config.redis.url).unwrap();
let redis_pool   = RedisPool::new(redis_config, None, None, None, 6).unwrap();
redis_pool.connect();
let session_store = RedisStore::new(redis_pool);
```

Ajouter dans `AppConfig` :

```rust
pub redis: RedisConfig,

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}
```

**Aucun autre fichier ne change** — handlers, middleware, templates, domaine sont inchangés.

---

## Conventions

### Handlers

- Un handler = une responsabilité unique
- Retournent toujours `Result<impl IntoResponse, AppError>`
- Aucune logique métier dans un handler — déléguer au service applicatif
- Récupérer l'utilisateur courant via `AuthSession` injecté par axum-login

### Domaine

- Les Value Objects ont un constructeur privé et un smart constructor `new()` retournant `Result<Self, DomainError>`
- Les agrégats n'exposent jamais de référence mutable vers leur état interne
- `DomainError` est un enum exhaustif avec `thiserror`
- Aucune dépendance framework dans `domain/`

### Templates

- Un template de **page complète** pour le premier chargement
- Des templates de **fragments** pour les réponses HTMX (swap partiel)
- Les structs de template portent uniquement des **view models** — pas d'entités domaine directement

### Réponses HTMX spéciales

```rust
// Redirect après action
Response::builder()
    .header("HX-Redirect", "/dashboard")
    .body(Body::empty()).unwrap()

// Refresh de la page courante
Response::builder()
    .header("HX-Refresh", "true")
    .body(Body::empty()).unwrap()

// Trigger d'événement côté client
Response::builder()
    .header("HX-Trigger", r#"{"showToast": "Sauvegardé"}"#)
    .body(Body::empty()).unwrap()
```
