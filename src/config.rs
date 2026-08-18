use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub email: EmailConfig,
    pub references: ReferencesConfig,
    pub log: LogConfig,
    pub host_domain: String,
    pub bypass_auth: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    /// Niveau de l'application — `error`, `warn`, `info`, `debug`, `trace` — et
    /// **rien d'autre** : il devient le niveau de la cible `kreek`. Une
    /// directive entière (`kreek::app::players=debug`) donnerait
    /// `kreek=kreek::app::players=debug`, que le filtre refuse.
    ///
    /// Le ciblage fin passe par `RUST_LOG`, qui supplante ce réglage quand il
    /// est posé — c'est l'échappatoire d'investigation, qui ouvre un BC le
    /// temps d'un incident sans toucher à la configuration déployée.
    pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String, // pas de défaut — obligatoire via env
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    /// Délai au bout duquel Postgres tue une session restée **oisive dans une
    /// transaction**.
    ///
    /// Sans lui, une transaction que personne ne referme — le cas d'une requête
    /// annulée en vol, cf. carte 317 — garde ses verrous jusqu'au redémarrage du
    /// serveur et bloque tout ce qui les demande.
    ///
    /// Ne frappe que l'oisiveté **dans** une transaction : une migration longue
    /// au démarrage, elle, est active et ne risque rien. C'est ce qui distingue
    /// ce réglage de `statement_timeout` et de `transaction_timeout`, écartés
    /// pour cette raison.
    pub idle_in_transaction_timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub token_ttl_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmailConfig {
    pub provider: EmailProvider,
    /// Optionnelle : `provider = "console"` n'en a pas besoin. Son absence
    /// avec `provider = "resend"` est rattrapée au démarrage, pas à l'envoi.
    #[serde(default)]
    pub api_key: String,
    pub from: String,
    pub from_name: String,
}

/// Qui expédie réellement les emails. `Console` écrit sur la sortie standard :
/// c'est ce que veut la suite e2e, dont le parcours « mot de passe oublié »
/// appellerait sinon l'API Resend à chaque exécution.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmailProvider {
    Console,
    Resend,
}

/// Répertoire des données de référence (règles de jeu), lu au démarrage.
/// Les données ne sont pas embarquées dans le binaire : chaque déploiement
/// fournit son propre jeu de règles via `REFERENCES__DIR`.
#[derive(Debug, Deserialize, Clone)]
pub struct ReferencesConfig {
    pub dir: String,
}

impl AppConfig {
    /// Configuration du harnais de test de handler (carte 311).
    ///
    /// Écrite à la main plutôt que chargée : `load()` lit `.env.<profil>` et
    /// l'environnement, donc un test dépendrait de la machine qui l'exécute.
    ///
    /// `bypass_auth` est **faux** : le harnais pose sa propre couche
    /// d'identité, et laisser le bypass actif ferait passer des tests
    /// d'autorisation en connectant un utilisateur que le test n'a pas choisi.
    #[cfg(test)]
    pub fn for_tests() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
                request_timeout_ms: 30_000,
            },
            database: DatabaseConfig {
                // Inutilisée : `compose()` reçoit la `PgPool` de `#[sqlx::test]`.
                url: String::new(),
                max_connections: 5,
                min_connections: 1,
                acquire_timeout_seconds: 5,
                idle_timeout_seconds: 600,
                idle_in_transaction_timeout_seconds: 15,
            },
            auth: AuthConfig {
                token_ttl_seconds: 3600,
            },
            // Inutilisé : le harnais n'installe pas de souscripteur.
            log: LogConfig {
                level: "info".into(),
            },
            email: EmailConfig {
                provider: EmailProvider::Console,
                api_key: String::new(),
                from: "test@example.test".into(),
                from_name: "Kreek".into(),
            },
            references: ReferencesConfig {
                dir: "assets/references.example".into(),
            },
            host_domain: "http://localhost".into(),
            bypass_auth: false,
        }
    }

    pub fn load() -> Result<Self, config::ConfigError> {
        // 1. Charge le fichier .env (ignoré silencieusement s'il n'existe pas)
        let env = env::var("EXEC_PROFILE").unwrap_or_else(|_| "dev".to_string());
        let dot_env_file = format!(".env.{}", env);
        dotenvy::from_filename(&dot_env_file).ok();

        // 2. Détermine l'environnement
        let env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        let config = config::Config::builder()
            // Couche 1 — valeurs par défaut
            .add_source(config::File::with_name("config/default").required(true))
            // Couche 2 — surcharge par environnement (optionnelle)
            .add_source(config::File::with_name(&format!("config/{}", env)).required(false))
            // Couche 3 — variables d'environnement (priorité maximale)
            //
            // **Sans préfixe** : `Environment::default()` n'en pose aucun, donc
            // la clé est la section elle-même. Un `APP__DATABASE__URL` se
            // lirait `app.database.url`, chemin qui n'existe pas — et serait
            // ignoré en silence.
            //   DATABASE__URL → database.url
            //   SERVER__PORT  → server.port
            //   LOG__LEVEL    → log.level
            .add_source(
                config::Environment::default()
                    .separator("__") // double underscore pour les niveaux imbriqués
                    .try_parsing(true), // parse les types : "8080" → u16, "true" → bool
            )
            .build()?;

        config.try_deserialize::<AppConfig>()
    }

    /// Adresse complète pour le binding du serveur
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
