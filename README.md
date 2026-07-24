# kreek

Application web Rust (Axum + HTMX), architecture orientée domaine, rendu HTML serveur (Askama).

## Démarrage rapide

1. Copier la config d'exemple et renseigner tes valeurs :
   ```bash
   cp .env.example .env
   ```
2. Lancer les migrations :
   ```bash
   sqlx migrate run
   ```
3. Créer le compte de développement (voir ci-dessous), puis démarrer le serveur.

## Compte de seed (développement)

Le dépôt ne versionne **aucune donnée réelle**. Un compte de développement
synthétique est fourni via `scripts/seed_accounts.example.json`. Le fichier
réel `scripts/seed_accounts.json` est ignoré par git — crée-le à partir de
l'exemple :

```bash
cp scripts/seed_accounts.example.json scripts/seed_accounts.json
cargo run -- seed-accounts --input scripts/seed_accounts.json
```

> Le compte d'exemple porte `legacy_id: 1`, requis par le mode `BYPASS_AUTH`
> (voir ci-dessous) et par les tests e2e.

## BYPASS_AUTH — développement uniquement ⚠️

`BYPASS_AUTH=true` connecte automatiquement l'utilisateur `legacy_id=1` sans
authentification. **À n'utiliser qu'en développement local.** Ne jamais
l'activer dans un environnement exposé — c'est un contournement total de
l'authentification. La valeur par défaut est `false` (`config/default.toml`).
