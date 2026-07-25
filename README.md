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

## Données de règles

kreek est un moteur de gestion de ligue : il ne contient aucune règle de jeu.
Il lit un ruleset au démarrage, dont le format est décrit dans
[`docs/reference-data-schema.md`](docs/reference-data-schema.md).

Un jeu de démonstration fictif et complet est fourni dans
`assets/references.example/` — c'est le défaut, l'application tourne sans
configuration. Pour jouer avec un autre corpus de règles, fournir son propre
répertoire via `REFERENCES__DIR`.

La licence AGPL-3.0 couvre le code de kreek. Elle ne s'étend pas aux données de
règles que l'exploitant fournit, dont il lui appartient de vérifier qu'il a le
droit de les utiliser.

Projet non officiel, sans affiliation avec Games Workshop.

## License

Kreek is licensed under **AGPL-3.0-or-later**. See [`LICENSE`](LICENSE).

For proprietary embedding or commercial redistribution without AGPL
obligations, a commercial license is available — contact
bertrand.begouin@wedge-digital.com.
