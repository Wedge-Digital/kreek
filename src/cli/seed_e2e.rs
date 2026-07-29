//! Seed minimal permettant à la suite e2e de démarrer sur une base neuve.
//!
//! Les tests créent eux-mêmes compétitions, équipes et joueurs ; il leur
//! manque seulement un utilisateur connectable par `bypass_auth`, un space, et
//! assez de coachs pour alimenter les sélecteurs. Tout est synthétique : ce
//! seed remplace la dépendance historique aux extraits legacy non versionnés.

use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use sqlx::PgPool;
use ulid::Ulid;

/// Nom et legacy_id imposés : `bypass_auth` connecte l'utilisateur legacy_id=1,
/// et `tests/e2e/competition_lifecycle.py` le recherche par ce nom exact.
const DEV_COACH_NAME: &str = "DevCoach";
const DEV_COACH_EMAIL: &str = "dev@example.test";
const DEV_COACH_LEGACY_ID: i32 = 1;

const SEED_PASSWORD: &str = "changeme-dev-only";
const SPACE_NAME: &str = "Espace E2E";

/// `space_icon_path` est relu à travers `CloudinaryImage`, qui n'accepte qu'une
/// URL `res.cloudinary.com` — un chemin local ferait échouer la lecture du space.
const SPACE_ICON: &str = "https://res.cloudinary.com/demo/image/upload/v1/sample.jpg";

/// Les parcours de compétition sélectionnent les coachs par index dans le
/// picker (jusqu'à 4 équipes), et les tests du coach-selector vérifient
/// l'exclusion des résultats déjà choisis : il en faut une poignée.
const EXTRA_COACHES: u8 = 11;

type SeedResult<T> = Result<T, Box<dyn std::error::Error>>;

pub async fn execute(pool: &PgPool) -> SeedResult<()> {
    let space_id = upsert_space(pool).await?;

    let dev_id = seed_coach(
        pool,
        DEV_COACH_NAME,
        DEV_COACH_EMAIL,
        Some(DEV_COACH_LEGACY_ID),
    )
    .await?;
    link_member(pool, &space_id, &dev_id, SpaceProfile::SpaceAdmin).await?;

    for n in 1..=EXTRA_COACHES {
        let id = seed_coach(pool, &coach_name(n), &coach_email(n), None).await?;
        link_member(pool, &space_id, &id, SpaceProfile::SpaceUser).await?;
    }

    tracing::info!(
        "seed-e2e: space « {SPACE_NAME} » ({space_id}) + {} coachs",
        EXTRA_COACHES + 1
    );
    Ok(())
}

fn coach_name(n: u8) -> String {
    format!("E2E Coach {n:02}")
}

fn coach_email(n: u8) -> String {
    format!("e2e-coach-{n:02}@example.test")
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

/// Crée le compte et sa projection dans le cache du BC spaces, en propageant
/// l'identifiant réellement retenu en base — c'est lui qui relie les trois tables.
async fn seed_coach(
    pool: &PgPool,
    name: &str,
    email: &str,
    legacy_id: Option<i32>,
) -> SeedResult<String> {
    let id = upsert_user(pool, name, email, legacy_id).await?;
    upsert_user_cache(pool, &id, name, email).await?;
    Ok(id)
}

/// `DO UPDATE` et non `DO NOTHING` : c'est ce qui garantit que `RETURNING` rende
/// l'identifiant aussi bien à la création qu'au conflit. Sans lui, une seconde
/// exécution ignorerait quel id rattacher au space, et créerait des adhésions
/// orphelines.
async fn upsert_user(
    pool: &PgPool,
    name: &str,
    email: &str,
    legacy_id: Option<i32>,
) -> SeedResult<String> {
    let hash = hash_password(SEED_PASSWORD).map_err(|e| format!("hash « {name} » : {e}"))?;
    let row: (String,) = sqlx::query_as(
        r#"
        INSERT INTO auth__users (id, coach_name, email, password_hash, legacy_id, created_at)
        VALUES ($1, $2, $3, $4, $5, now())
        ON CONFLICT (coach_name)
        DO UPDATE SET email = EXCLUDED.email, legacy_id = EXCLUDED.legacy_id
        RETURNING id
        "#,
    )
    .bind(Ulid::new().to_string())
    .bind(name)
    .bind(email)
    .bind(&hash)
    .bind(legacy_id)
    .fetch_one(pool)
    .await
    .map_err(|e| conflict_hint(name, e))?;
    Ok(row.0)
}

async fn upsert_user_cache(pool: &PgPool, id: &str, name: &str, email: &str) -> SeedResult<()> {
    sqlx::query(
        r#"
        INSERT INTO spaces__user_cache (id, coach_name, email, created_at)
        VALUES ($1, $2, $3, now())
        ON CONFLICT (id)
        DO UPDATE SET coach_name = EXCLUDED.coach_name, email = EXCLUDED.email
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(email)
    .execute(pool)
    .await
    .map_err(|e| conflict_hint(name, e))?;
    Ok(())
}

async fn upsert_space(pool: &PgPool) -> SeedResult<String> {
    let row: (String,) = sqlx::query_as(
        r#"
        INSERT INTO spaces (id, space_name, space_icon_path, created_at)
        VALUES ($1, $2, $3, now())
        ON CONFLICT (space_name)
        DO UPDATE SET space_icon_path = EXCLUDED.space_icon_path
        RETURNING id
        "#,
    )
    .bind(Ulid::new().to_string())
    .bind(SPACE_NAME)
    .bind(SPACE_ICON)
    .fetch_one(pool)
    .await
    .map_err(|e| conflict_hint(SPACE_NAME, e))?;
    Ok(row.0)
}

async fn link_member(
    pool: &PgPool,
    space_id: &str,
    coach_id: &str,
    profile: SpaceProfile,
) -> SeedResult<()> {
    sqlx::query(
        r#"
        INSERT INTO spaces__user_space (space_id, coach_id, profile, created_at)
        VALUES ($1, $2, $3, now())
        ON CONFLICT (space_id, coach_id) DO UPDATE SET profile = EXCLUDED.profile
        "#,
    )
    .bind(space_id)
    .bind(coach_id)
    .bind(profile.as_str())
    .execute(pool)
    .await?;
    Ok(())
}

/// `email` et `legacy_id` portent aussi des contraintes d'unicité : si une
/// donnée préexistante les détient, l'upsert échoue sur une contrainte dont le
/// message brut de Postgres n'explique pas la cause dans ce contexte.
fn conflict_hint(label: &str, e: sqlx::Error) -> String {
    match e.as_database_error().and_then(|d| d.constraint()) {
        Some(c) => format!(
            "seed de « {label} » impossible : la contrainte {c} est déjà détenue \
             par une autre ligne. Repartir d'une base propre (make reset_db)."
        ),
        None => format!("seed de « {label} » : {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn count(pool: &PgPool, table: &str) -> i64 {
        sqlx::query_scalar(&format!("select count(*) from {table}"))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn counts(pool: &PgPool) -> (i64, i64, i64, i64) {
        (
            count(pool, "auth__users").await,
            count(pool, "spaces").await,
            count(pool, "spaces__user_cache").await,
            count(pool, "spaces__user_space").await,
        )
    }

    #[sqlx::test]
    async fn seeds_one_space_and_twelve_coaches(pool: PgPool) {
        execute(&pool).await.unwrap();
        assert_eq!(counts(&pool).await, (12, 1, 12, 12));
    }

    #[sqlx::test]
    async fn dev_coach_is_admin_of_the_space_with_legacy_id_one(pool: PgPool) {
        execute(&pool).await.unwrap();
        let profile: String = sqlx::query_scalar(
            "select us.profile from spaces__user_space us
             join auth__users u on u.id = us.coach_id
             where u.legacy_id = 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(profile, SpaceProfile::SpaceAdmin.as_str());
    }

    /// L'invariant que protège le `RETURNING id` : rejouer le seed ne doit ni
    /// dupliquer, ni casser le lien entre les trois tables.
    #[sqlx::test]
    async fn seeding_twice_is_idempotent_and_keeps_ids_consistent(pool: PgPool) {
        execute(&pool).await.unwrap();
        let first = counts(&pool).await;

        execute(&pool).await.unwrap();
        assert_eq!(counts(&pool).await, first);

        let orphans: i64 = sqlx::query_scalar(
            "select count(*) from spaces__user_space us
             where not exists (select 1 from auth__users u  where u.id = us.coach_id)
                or not exists (select 1 from spaces__user_cache c where c.id = us.coach_id)",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(orphans, 0, "adhésions orphelines après un second seed");
    }
}
