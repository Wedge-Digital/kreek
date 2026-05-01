use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server:   ServerConfig,
    pub database: DatabaseConfig,
    pub auth:     AuthConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host:               String,
    pub port:               u16,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url:                     String,  // pas de défaut — obligatoire via env
    pub max_connections:         u32,
    pub min_connections:         u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds:    u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub token_ttl_seconds:  u64,
}


impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        // 1. Charge le fichier .env (ignoré silencieusement s'il n'existe pas)
        let env = env::var("EXEC_PROFILE").unwrap_or_else(|_| "dev".to_string());
        let dot_env_file = format!(".env.{}", env);
        dotenvy::from_filename(&dot_env_file).ok();

        // 2. Détermine l'environnement
        let env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        let config = config::Config::builder()
            // Couche 1 — valeurs par défaut
            .add_source(
                config::File::with_name("config/default")
                    .required(true)
            )
            // Couche 2 — surcharge par environnement (optionnelle)
            .add_source(
                config::File::with_name(&format!("config/{}", env))
                    .required(false)
            )
            // Couche 3 — variables d'environnement (priorité maximale)
            // APP__DATABASE__URL → database.url
            // APP__SERVER__PORT  → server.port
            .add_source(
                config::Environment::default()
                    .separator("__")       // double underscore pour les niveaux imbriqués
                    .try_parsing(true)     // parse les types : "8080" → u16, "true" → bool
            )
            .build()?;

        config.try_deserialize::<AppConfig>()
    }

    /// Adresse complète pour le binding du serveur
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}