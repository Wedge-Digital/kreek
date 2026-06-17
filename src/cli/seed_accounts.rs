use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use serde::Deserialize;
use sqlx::PgPool;
use ulid::Ulid;

#[derive(Deserialize)]
struct SeedAccount {
    coach_name: String,
    password: String,
    email: String,
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub async fn execute(pool: &PgPool, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(input)
        .map_err(|e| format!("Cannot read {input}: {e}"))?;
    let accounts: Vec<SeedAccount> = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON in {input}: {e}"))?;

    for account in &accounts {
        let hash = hash_password(&account.password)
            .map_err(|e| format!("Hash error for {}: {e}", account.coach_name))?;

        let id = Ulid::new().to_string();

        let result = sqlx::query(
            r#"
            INSERT INTO auth__users (id, coach_name, email, password_hash, created_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (coach_name)
            DO UPDATE SET password_hash = EXCLUDED.password_hash, email = EXCLUDED.email
            "#,
        )
        .bind(&id)
        .bind(&account.coach_name)
        .bind(&account.email)
        .bind(&hash)
        .execute(pool)
        .await?;

        if result.rows_affected() > 0 {
            tracing::info!("✓ {}: upserted", account.coach_name);
        }
    }

    tracing::info!("seed-accounts: {} accounts processed", accounts.len());
    Ok(())
}
