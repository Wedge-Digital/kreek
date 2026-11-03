//! Le magasin des sessions — Postgres, table `sessions` (carte 491).
//!
//! Il remplace un `DashMap` en mémoire : chaque redéploiement déconnectait tout
//! le monde en même temps, et rien n'était jamais purgé.
//!
//! Deux décorateurs enveloppent `PostgresStore`, de l'intérieur vers
//! l'extérieur :
//!
//! - [`HashedIds`] range chaque session sous l'**empreinte** de son
//!   identifiant. L'identifiant est la valeur même du cookie, non signée : sans
//!   ce décorateur, chaque ligne de la table — dans un dump, une sauvegarde —
//!   serait un cookie valide trente jours.
//! - [`Logged`] journalise chaque échec sous `kreek::`. Le 500 que rend alors la
//!   couche de session est écrit par `tower_sessions` et `axum_login`, sur
//!   leurs propres cibles, que le filtre `kreek=…` n'active pas : sans lui, une
//!   base indisponible produirait des 500 sans une ligne de journal.

use std::time::Duration;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tower_sessions::{
    session::{Id, Record},
    session_store, ExpiredDeletion, SessionStore,
};
use tower_sessions_sqlx_store::PostgresStore;

/// Le magasin que reçoit la couche de session.
pub type AppSessionStore = Logged<HashedIds<PostgresStore>>;

/// La table créée par `migrations/20261102000002_sessions.sql`.
const TABLE: &str = "sessions";

/// Une heure : les sessions durent trente jours, la précision de la purge ne
/// compte pas — seulement qu'elle ait lieu.
pub const PURGE_PERIOD: Duration = Duration::from_secs(3600);

/// Le magasin de l'application, sur la table `sessions`.
pub fn app_session_store(pool: sqlx::PgPool) -> AppSessionStore {
    Logged(HashedIds(postgres_store(pool)))
}

fn postgres_store(pool: sqlx::PgPool) -> PostgresStore {
    PostgresStore::new(pool)
        .with_schema_name("public")
        .and_then(|store| store.with_table_name(TABLE))
        .expect("noms de schéma et de table de sessions valides")
}

/// L'empreinte sous laquelle une session est rangée : SHA-256 de l'identifiant,
/// tronquée aux 128 bits que porte un [`Id`].
///
/// Ni sel ni dérivation lente : l'identifiant est déjà 128 bits tirés au
/// hasard, il n'y a pas de dictionnaire à parcourir.
pub fn hashed(id: &Id) -> Id {
    let digest = Sha256::digest(id.0.to_le_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Id(i128::from_le_bytes(bytes))
}

/// Range chaque session sous [`hashed`] de son identifiant.
///
/// `tower-sessions` ne connaît que l'identifiant en clair : c'est lui qu'il
/// passe à `load`, `save` et `delete`, et celui qu'il relit dans `record.id`
/// après `create` (le cookie) et dans `cycle_id`. Le décorateur traduit donc à
/// l'aller et **rend l'identifiant en clair** au retour de `load`.
#[derive(Clone, Debug)]
pub struct HashedIds<S>(pub S);

#[async_trait]
impl<S: SessionStore + Clone> SessionStore for HashedIds<S> {
    /// La collision se résout sur l'identifiant **en clair** : c'est lui que
    /// porte le cookie. Si le magasin intérieur a dû changer l'empreinte — une
    /// collision d'empreintes, d'une probabilité de 2⁻¹²⁸ —, la ligne qu'il a
    /// créée ne correspond à aucun cookie : elle est retirée, et on retire un
    /// identifiant.
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        loop {
            let wanted = hashed(&record.id);
            let mut stored = with_id(record, wanted);
            self.0.create(&mut stored).await?;
            if stored.id == wanted {
                return Ok(());
            }
            self.0.delete(&stored.id).await?;
            record.id = Id::default();
        }
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        self.0.save(&with_id(record, hashed(&record.id))).await
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let loaded = self.0.load(&hashed(session_id)).await?;
        Ok(loaded.map(|record| with_id(&record, *session_id)))
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        self.0.delete(&hashed(session_id)).await
    }
}

fn with_id(record: &Record, id: Id) -> Record {
    Record {
        id,
        ..record.clone()
    }
}

/// Journalise sous `kreek::` chaque échec du magasin, puis le rend tel quel.
#[derive(Clone, Debug)]
pub struct Logged<S>(pub S);

#[async_trait]
impl<S: SessionStore + Clone> SessionStore for Logged<S> {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        logged("create", self.0.create(record).await)
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        logged("save", self.0.save(record).await)
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        logged("load", self.0.load(session_id).await)
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        logged("delete", self.0.delete(session_id).await)
    }
}

fn logged<T>(operation: &str, result: session_store::Result<T>) -> session_store::Result<T> {
    if let Err(erreur) = &result {
        tracing::error!(
            operation,
            %erreur,
            "magasin de sessions en échec — la requête rendra un 500"
        );
    }
    result
}

/// Lance la purge horaire des sessions expirées, pour la durée du processus.
///
/// Une boucle à nous plutôt que `continuously_delete_expired` : celle de la
/// crate s'arrête au premier échec, et une coupure de base d'une minute aurait
/// tué la purge jusqu'au déploiement suivant, sans un mot.
pub fn spawn_expired_sessions_purge(pool: sqlx::PgPool) {
    let store = postgres_store(pool);
    tracing::info!(
        periode_s = PURGE_PERIOD.as_secs(),
        "purge des sessions expirées programmée"
    );
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(PURGE_PERIOD);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            purge_expired_sessions(&store).await;
        }
    });
}

/// Un tour de purge. Un échec est journalisé et n'arrête rien : le tour suivant
/// réessaie.
///
/// La purge porte sur `expiry_date`, que [`HashedIds`] ne touche pas : elle
/// s'adresse donc directement au magasin intérieur.
pub async fn purge_expired_sessions(store: &impl ExpiredDeletion) {
    match store.delete_expired().await {
        Ok(()) => tracing::debug!("purge des sessions expirées effectuée"),
        Err(erreur) => tracing::warn!(
            %erreur,
            "purge des sessions expirées en échec — nouvel essai au prochain tour"
        ),
    }
}

/// Le magasin intérieur, pour les tests qui lisent la table sans décorateur.
#[cfg(test)]
pub fn raw_postgres_store(pool: sqlx::PgPool) -> PostgresStore {
    postgres_store(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_differs_from_the_id() {
        let id = Id::default();
        assert_eq!(hashed(&id), hashed(&id));
        assert_ne!(hashed(&id), id);
    }

    #[test]
    fn two_ids_have_two_hashes() {
        assert_ne!(hashed(&Id(1)), hashed(&Id(2)));
    }

    fn record(id: Id) -> Record {
        Record {
            id,
            data: Default::default(),
            expiry_date: time::OffsetDateTime::now_utc() + time::Duration::days(1),
        }
    }

    /// L'aller-retour : rangée sous l'empreinte, relue sous l'identifiant en
    /// clair — c'est lui que `tower-sessions` reprend pour `cycle_id`.
    #[tokio::test]
    async fn stores_under_the_hash_and_loads_back_the_clear_id() {
        let inner = tower_sessions::MemoryStore::default();
        let store = HashedIds(inner.clone());
        let mut rec = record(Id::default());
        store.create(&mut rec).await.unwrap();

        assert!(inner.load(&rec.id).await.unwrap().is_none());
        assert!(inner.load(&hashed(&rec.id)).await.unwrap().is_some());
        let loaded = store.load(&rec.id).await.unwrap().expect("session relue");
        assert_eq!(loaded.id, rec.id);
    }

    /// Une collision d'empreintes : le magasin intérieur change l'identifiant
    /// rangé. Le décorateur doit retirer cette ligne orpheline, tirer un nouvel
    /// identifiant en clair, et laisser intacte la session déjà rangée.
    #[tokio::test]
    async fn resolves_a_hash_collision_on_the_clear_id() {
        let inner = tower_sessions::MemoryStore::default();
        let clear = Id::default();
        let mut occupant = record(hashed(&clear));
        inner.create(&mut occupant).await.unwrap();

        let mut rec = record(clear);
        HashedIds(inner.clone()).create(&mut rec).await.unwrap();

        assert_ne!(rec.id, clear, "l'identifiant du cookie doit changer");
        assert!(inner.load(&hashed(&rec.id)).await.unwrap().is_some());
        assert!(inner.load(&hashed(&clear)).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn save_and_delete_address_the_hash() {
        let inner = tower_sessions::MemoryStore::default();
        let store = HashedIds(inner.clone());
        let rec = record(Id::default());
        store.save(&rec).await.unwrap();
        assert!(inner.load(&hashed(&rec.id)).await.unwrap().is_some());

        store.delete(&rec.id).await.unwrap();
        assert!(inner.load(&hashed(&rec.id)).await.unwrap().is_none());
    }
}
