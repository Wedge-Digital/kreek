# Commande CLI `seed-accounts`

**Priorité : haute**
**Dépend de :** `74-cli-infrastructure.md`
**Contexte :** infrastructure applicative

## Objectif

Implémenter la commande `seed-accounts` qui lit un fichier JSON de comptes dev, hashe les mots de passe en Argon2 (identique à l'app), et upsert en base. Idempotent : si le compte existe (par `coach_name`), met à jour le mot de passe et l'email.

---

## Conception

### Fichier JSON : `scripts/seed_accounts.json`

```json
[
  {
    "coach_name": "Bagouze",
    "password": "changeme-dev-only",
    "email": "dev@example.test"
  }
]
```

Versionné dans le repo — ce sont des comptes de dev local uniquement.

### Handler de commande

```rust
pub async fn seed_accounts(pool: &PgPool, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(input)?;
    let accounts: Vec<SeedAccount> = serde_json::from_str(&content)?;

    for account in &accounts {
        let hash = hash_password(&account.password)?;
        upsert_account(pool, &account.coach_name, &account.email, &hash).await?;
    }
    Ok(())
}
```

### Hash Argon2

Réutilise le même algorithme que `register_new_account.rs` :

```rust
fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}
```

### Upsert SQL

```sql
INSERT INTO auth__users (id, coach_name, email, password_hash, created_at)
VALUES ($1, $2, $3, $4, now())
ON CONFLICT (coach_name)
DO UPDATE SET password_hash = $4, email = $3
```

Génère un ULID pour l'`id` si c'est une insertion.

### Intégration Makefile

```makefile
seed_accounts:
	cargo run -- seed-accounts
```

Optionnellement appelé dans `init_db` après les imports.

---

## Checklist

- [ ] Créer `scripts/seed_accounts.json` avec le compte Bagouze
- [ ] Implémenter le handler `seed_accounts` dans un module dédié (`src/cli/seed_accounts.rs` ou similaire)
- [ ] Hash Argon2 identique à l'app (crate `argon2`)
- [ ] Upsert SQL : insert ou update password_hash + email sur conflit `coach_name`
- [ ] Câbler dans le `match Command` de `main.rs`
- [ ] Ajouter target `seed_accounts` dans le Makefile
- [ ] Ajouter l'appel dans `init_db` (optionnel, après les imports)
- [ ] Tester : `cargo run -- seed-accounts` puis login avec Bagouze/changeme-dev-only
