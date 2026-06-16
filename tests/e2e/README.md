# Tests E2E (Playwright)

Tests bout-en-bout pilotant un vrai navigateur (Chromium) contre le serveur
kreek lancé en dev. Complémentaires aux tests `cargo test` — ils valident le
rendu HTML/HTMX/Alpine réellement produit dans un navigateur.

## Prérequis

- Le serveur kreek doit déjà tourner en local avec `BYPASS_AUTH=true`
  (ex: `ENV=dev cargo run`, ou via `.env.dev`). Les tests ne le démarrent pas
  eux-mêmes — ils échouent immédiatement et clairement si rien ne répond sur
  `http://localhost:3210`.
- `uv` installé.

## Installation (une fois)

```bash
cd tests/e2e
uv sync
uv run playwright install chromium
```

## Lancer les tests

```bash
cd tests/e2e
uv run pytest -v
```

Ou depuis la racine : `make e2e`.

## Configuration

| Variable | Défaut | Usage |
|---|---|---|
| `E2E_BASE_URL` | `http://localhost:3210` | URL du serveur dev |
| `E2E_SPACE_ID` | `01KV65QPBK151AJTSAMB8BE6SS` | Space appartenant à l'utilisateur `legacy_id=1`, connecté automatiquement par `bypass_auth` |

## Portée actuelle

- `test_coach_selector.py` : widget coach-search multi-select sur la création
  de compétition (phase 1) — chargement, sélection multiple, badges,
  exclusion des résultats, suppression, transmission de `admin_ids` au
  serveur, absence d'erreurs console JS (vérifié automatiquement sur chaque
  test via une fixture autouse).
