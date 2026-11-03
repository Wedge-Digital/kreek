-- Le magasin de sessions persistant (carte 491).
--
-- Les colonnes sont exactement celles qu'interrogent les requêtes de
-- `tower_sessions_sqlx_store::PostgresStore` : elles ne se renomment pas.
--
-- La table est créée ici plutôt que par `PostgresStore::migrate()`, qui pose un
-- schéma `tower_sessions` (droit `CREATE SCHEMA` exigé du rôle applicatif) et
-- échappe au `MIGRATOR` du démarrage.
--
-- `id` n'est **pas** la valeur du cookie : c'est son empreinte SHA-256,
-- tronquée à 128 bits (`common::session_store::HashedIds`). Une ligne volée —
-- un dump, une sauvegarde — ne se rejoue donc pas comme cookie.
--
-- Pas de préfixe de BC : la table appartient à l'hôte, comme `event_log`.
-- Pas d'index sur `expiry_date` : la purge horaire parcourt quelques milliers
-- de lignes au plus.
CREATE TABLE sessions (
    id          TEXT PRIMARY KEY NOT NULL,
    data        BYTEA NOT NULL,
    expiry_date TIMESTAMPTZ NOT NULL
);
