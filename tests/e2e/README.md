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
| `E2E_SPACE_ID` | résolu dynamiquement | Space ciblé par les tests. Par défaut, résolu au lancement de la suite via `GET /app/spaces` (premier espace de l'utilisateur `legacy_id=1`, connecté automatiquement par `bypass_auth`) — robuste à un `make init_db` qui régénère des ULIDs aléatoires. Surcharger uniquement pour cibler un espace précis. |
| `E2E_COMPETITION_ID` / `E2E_SEASON_ID` | `E2E_SPACE_ID` | Utilisés seulement par `competition_rules_url` (la page accepte des IDs inexistants) |

**Important** : `seed_space_members.py` (appelé par `make init_db WITH_SEED=1`) affecte 100 coachs **aléatoires** par espace — rien ne garantit que l'utilisateur `legacy_id=1` en fasse partie après un seed donné. Si la résolution dynamique échoue ("Aucun space_id trouvé"), relancer `./scripts/seed_space_members.sh` (idempotent, ré-tire aléatoirement) jusqu'à ce qu'il soit inclus.

## Portée actuelle

- `test_coach_selector.py` : widget coach-search multi-select sur la création
  de compétition (phase 1) — chargement, sélection multiple, badges,
  exclusion des résultats, suppression, transmission de `admin_ids` au
  serveur, absence d'erreurs console JS (vérifié automatiquement sur chaque
  test via une fixture autouse).
- `test_phase2_pickers.py` : widgets roster/inducement/star-player picker sur
  la phase 2 ("Règles") — chargement des 3 widgets par tier, sélection
  "tout sélectionné" par défaut sur un tier neuf, **indépendance de l'état
  entre tiers** (régression-clé : ces widgets utilisaient un store Alpine
  global avant correction, ce qui faisait fuiter la sélection d'un tier vers
  les autres), toggle d'un chip.
- `test_full_competition_creation_flow.py` : parcours complet phase 1 → 2 → 3
  (informations + admin + logo → règles + tiers → structure), avec une
  vraie compétition/saison persistée en base (nom unique par exécution).
  Couvre la navigation réelle entre phases (HX-Redirect natif d'htmx pour
  1→2, navigation SPA `htmx.ajax` + `pushState` pour 2→3) — pas seulement
  chaque phase isolément.
- `test_phase4_invitations.py` : phase 4 ("Invitations") — mode d'accès (2
  options), validation des inscriptions (Oui/Non), widget coach-search
  réintégré pour l'invitation, visibilité conditionnelle de la section
  d'invitation selon le mode d'accès, récap en phase 5. Traverse
  obligatoirement le vrai parcours 1→2→3→4 : la régression corrigée (un
  déséquilibre de balises `<div>` qui empêchait le `<script>` de la page de
  s'exécuter) ne se manifestait que lors de la navigation SPA réelle depuis
  la phase 3, pas via un accès direct à l'URL.
- `test_team_detail_state_banner.py` : bandeau d'état contextuel de la page
  de détail d'équipe — rapport en cours (lien de reprise), phase
  d'amélioration/recrutement/renvois (bouton de validation → transition réelle
  de phase, badge d'en-tête mis à jour), retour à "Prête à jouer" (bandeau
  impression). Séquence pilotée de bout en bout par de vraies actions
  applicatives (création/publication d'un rapport de match puis les 3 routes
  de validation de phase) sur une même équipe. En attente d'inscription
  vérifié dynamiquement (skip si absent de la base seedée). Retraite
  temporaire/off-season non testés — aucune voie applicative ne permet
  actuellement de les atteindre (carte 46, admin override-phase, non faite).
- `test_player_spp_spending.py` : slot droit de la fiche joueur (journal en
  lecture seule vs panneau de dépense de SPP) — bascule automatique selon la
  phase `PlayerImprovement` réelle de l'équipe (publication d'un vrai rapport
  de match), achat d'une compétence (réserve SPP décrémentée, tag de
  compétence acquise), joueur sans SPP (aucune compétence achetable),
  augmentation de caractéristique (stat + réserve mises à jour),
  incrémentation de `team_value` sur la fiche équipe (pipeline app event
  `players → teams`). Utilisateur non autorisé non testé ici : `bypass_auth`
  connecte toujours le même coach, incompatible avec ce scénario sans
  fabriquer un état dans l'event store (cf. docstring du fichier).
