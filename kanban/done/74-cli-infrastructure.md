# Infrastructure CLI — sous-commandes clap

**Priorité : haute (bloquant pour 75)**
**Dépend de :** rien
**Contexte :** infrastructure applicative

## Objectif

Ajouter `clap` et transformer le point d'entrée en CLI avec sous-commandes. La commande par défaut (`serve`) lance le serveur HTTP comme aujourd'hui. Les futures commandes admin (seed, migrations, etc.) s'ajoutent comme sous-commandes.

---

## Conception

### Dépendance

```toml
clap = { version = "4", features = ["derive"] }
```

### Enum de commandes

```rust
#[derive(clap::Parser)]
#[command(name = "kreek")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Lance le serveur HTTP (défaut)
    Serve,
    /// Seed les comptes utilisateurs depuis un fichier JSON
    SeedAccounts {
        #[arg(long, default_value = "scripts/seed_accounts.json")]
        input: String,
    },
}
```

### Refacto main.rs

- Extraire l'initialisation du pool PG dans une fonction `init_pool(cfg: &AppConfig) -> PgPool`
- Le `match` sur la commande appelle soit `run_server(pool, cfg)` soit `seed_accounts(pool, input)`
- `serve` est la commande par défaut si aucune sous-commande n'est fournie

### Usage

```bash
# Serveur (identique à aujourd'hui)
cargo run
cargo run -- serve

# Seed accounts
cargo run -- seed-accounts
cargo run -- seed-accounts --input scripts/seed_accounts.json
```

---

## Checklist

- [ ] Ajouter `clap = { version = "4", features = ["derive"] }` dans `Cargo.toml`
- [ ] Définir `Cli` + `Command` enum dans `main.rs`
- [ ] Extraire `init_pool()` de `main()`
- [ ] Encapsuler le démarrage serveur dans `Command::Serve`
- [ ] `Command::Serve` est le défaut quand aucune sous-commande n'est fournie
- [ ] `cargo run` fonctionne comme avant
- [ ] `cargo run -- --help` affiche les sous-commandes disponibles
