//! Les migrations de **données** — celles qui ont besoin du corpus de règles.
//!
//! # Pourquoi elles ne sont pas du SQL
//!
//! Le corpus vit hors du dépôt : `REFERENCES__DIR` est fourni par l'exploitant,
//! et `load_references` ne le lit qu'au milieu de `run_server`, bien après
//! `run_migrations`. Une migration SQL ne peut donc savoir ni quelles
//! compétences sont Élite, ni quels rosters portent « Lineman a vil prix » :
//! ces faits n'existent que dans des fichiers que la base ne voit pas.
//!
//! D'où cette seconde famille, écrite en Rust, qui reçoit les mêmes ports que
//! l'application — `AppState` est déjà construit quand elles s'exécutent.
//!
//! # Un échec refuse le démarrage
//!
//! Et non un avertissement suivi d'un démarrage normal. Servir des pages sur
//! des données à moitié migrées est pire qu'un déploiement qui s'arrête :
//! personne ne lira la ligne d'avertissement, tout le monde verra les valeurs
//! fausses. C'est la règle « une étape sautée doit échouer, pas rassurer ».
//!
//! # L'ordre est une donnée, pas un hasard
//!
//! Le registre est une liste, et il le reste. La migration des valeurs
//! d'équipe recalcule des sommes à partir des valeurs joueurs que celle des
//! compétences Élite vient de corriger : les inverser donnerait des valeurs
//! fausses, sans que rien ne le signale.
//!
//! # La marque et l'écriture sont atomiques
//!
//! La table de garde protège du rejeu **normal** ; elle ne protège pas d'une
//! interruption au milieu. Chaque migration écrit donc son nom dans la même
//! transaction que ses effets — c'est la règle des projections appliquée à un
//! autre objet. Une migration interrompue n'a rien écrit **et** n'est pas
//! marquée : elle repassera entière au démarrage suivant.

use crate::state::AppState;
use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

/// Une migration de données.
///
/// Elle reçoit la transaction dans laquelle elle doit écrire — c'est le
/// registre qui l'ouvre, la marque et la valide, de sorte qu'aucune migration
/// ne puisse oublier l'un des trois.
#[async_trait]
pub trait DataMigration: Send + Sync {
    /// Son identité en base. Ne change jamais : le renommer ferait rejouer la
    /// migration sur toutes les installations qui l'ont déjà appliquée.
    fn nom(&self) -> &'static str;

    /// Rend le nombre d'agrégats touchés — pour le journal, et pour qu'une
    /// migration qui ne trouve rien se distingue d'une migration qui a agi.
    async fn executer(
        &self,
        state: &AppState,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<usize, String>;
}

pub mod m001_bonus_elite;
pub mod m002_recalcul_valeurs_equipe;

/// Le registre, dans son ordre d'exécution.
///
/// L'ordre est une donnée : le recalcul des valeurs d'équipe somme les valeurs
/// joueurs que le bonus Élite vient de corriger. Inversés, il rendrait des
/// valeurs fausses sans que rien ne le signale.
fn registre() -> Vec<Box<dyn DataMigration>> {
    vec![
        Box::new(m001_bonus_elite::BonusElite),
        Box::new(m002_recalcul_valeurs_equipe::RecalculValeursEquipe),
    ]
}

/// Applique ce qui ne l'a pas encore été, puis rend la main. Refuse le
/// démarrage au premier échec.
pub async fn executer(state: &AppState, pool: &PgPool) {
    if let Err(e) = appliquer(state, pool, registre()).await {
        tracing::error!("migration de données en échec : {e}");
        panic!("migration de données en échec : {e}");
    }
}

/// Le cœur, séparé de [`executer`] pour être testable avec un registre à soi :
/// un test ne doit pas dépendre des migrations réelles, dont la liste change.
pub async fn appliquer(
    state: &AppState,
    pool: &PgPool,
    migrations: Vec<Box<dyn DataMigration>>,
) -> Result<(), String> {
    for m in migrations {
        if deja_appliquee(pool, m.nom()).await? {
            tracing::info!(migration = m.nom(), "migration de données déjà appliquée");
            continue;
        }

        let debut = std::time::Instant::now();
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

        let touches = m.executer(state, &mut tx).await?;

        sqlx::query("INSERT INTO applied_data_migrations (name) VALUES ($1)")
            .bind(m.nom())
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("marquage de « {} » : {e}", m.nom()))?;

        tx.commit().await.map_err(|e| e.to_string())?;

        tracing::info!(
            migration = m.nom(),
            agregats = touches,
            duree_ms = debut.elapsed().as_millis(),
            "migration de données appliquée"
        );
    }
    Ok(())
}

async fn deja_appliquee(pool: &PgPool, nom: &str) -> Result<bool, String> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM applied_data_migrations WHERE name = $1")
        .bind(nom)
        .fetch_one(pool)
        .await
        .map(|n| n > 0)
        .map_err(|e| format!("lecture du registre : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Compte ses exécutions, et écrit une ligne pour qu'on puisse constater
    /// qu'un échec ne laisse rien derrière lui.
    struct Factice {
        nom: &'static str,
        appels: Arc<AtomicUsize>,
        echoue: bool,
    }

    #[async_trait]
    impl DataMigration for Factice {
        fn nom(&self) -> &'static str {
            self.nom
        }

        async fn executer(
            &self,
            _state: &AppState,
            tx: &mut Transaction<'_, Postgres>,
        ) -> Result<usize, String> {
            self.appels.fetch_add(1, Ordering::SeqCst);
            sqlx::query("INSERT INTO applied_data_migrations (name) VALUES ($1)")
                .bind(format!("trace-{}", self.nom))
                .execute(&mut **tx)
                .await
                .map_err(|e| e.to_string())?;
            if self.echoue {
                return Err("échec délibéré".to_string());
            }
            Ok(1)
        }
    }

    fn factice(
        nom: &'static str,
        appels: &Arc<AtomicUsize>,
        echoue: bool,
    ) -> Box<dyn DataMigration> {
        Box::new(Factice {
            nom,
            appels: Arc::clone(appels),
            echoue,
        })
    }

    async fn etat(pool: sqlx::PgPool) -> AppState {
        crate::compose(crate::config::AppConfig::for_tests(), pool).await
    }

    async fn noms(pool: &PgPool) -> Vec<String> {
        sqlx::query_scalar("SELECT name FROM applied_data_migrations ORDER BY name")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    /// C'est tout l'objet de la table de garde : la seconde exécution ne
    /// rejoue rien.
    #[sqlx::test]
    async fn une_migration_appliquee_deux_fois_ne_s_execute_qu_une(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        let appels = Arc::new(AtomicUsize::new(0));

        appliquer(&state, &pool, vec![factice("m1", &appels, false)])
            .await
            .unwrap();
        appliquer(&state, &pool, vec![factice("m1", &appels, false)])
            .await
            .unwrap();

        assert_eq!(appels.load(Ordering::SeqCst), 1);
        assert_eq!(noms(&pool).await, vec!["m1", "trace-m1"]);
    }

    /// L'atomicité, et non la seule table de garde : une migration qui échoue
    /// après avoir écrit ne doit **rien** laisser — ni sa trace, ni sa marque.
    /// Sans cela, une interruption au milieu produirait une base à moitié
    /// migrée qui se croit à jour.
    #[sqlx::test]
    async fn une_migration_en_echec_ne_laisse_rien(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        let appels = Arc::new(AtomicUsize::new(0));

        let r = appliquer(&state, &pool, vec![factice("m2", &appels, true)]).await;

        assert!(r.is_err(), "l'échec doit remonter");
        assert_eq!(appels.load(Ordering::SeqCst), 1);
        assert!(
            noms(&pool).await.is_empty(),
            "ni la marque ni l'écriture ne doivent subsister"
        );
    }

    /// L'ordre du registre est une donnée : la migration des valeurs d'équipe
    /// lit ce que celle des compétences vient de corriger.
    #[sqlx::test]
    async fn les_migrations_s_executent_dans_l_ordre_du_registre(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        let appels = Arc::new(AtomicUsize::new(0));

        appliquer(
            &state,
            &pool,
            vec![
                factice("a-premiere", &appels, false),
                factice("b-seconde", &appels, false),
            ],
        )
        .await
        .unwrap();

        let vues: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM applied_data_migrations WHERE name LIKE 'trace-%' ORDER BY applied_at, name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(vues, vec!["trace-a-premiere", "trace-b-seconde"]);
    }

    /// Une migration en échec arrête la série : celles qui suivent liraient des
    /// données à moitié corrigées.
    #[sqlx::test]
    async fn un_echec_arrete_la_serie(pool: sqlx::PgPool) {
        let state = etat(pool.clone()).await;
        let appels = Arc::new(AtomicUsize::new(0));

        let r = appliquer(
            &state,
            &pool,
            vec![
                factice("qui-echoue", &appels, true),
                factice("qui-suit", &appels, false),
            ],
        )
        .await;

        assert!(r.is_err());
        assert_eq!(
            appels.load(Ordering::SeqCst),
            1,
            "la suivante ne doit pas tourner"
        );
    }
}
