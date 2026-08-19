# Tests E2E (Playwright)

Tests bout-en-bout pilotant un vrai navigateur (Chromium) contre le serveur
kreek lancé en dev. Complémentaires aux tests `cargo test` — ils valident le
rendu HTML/HTMX/Alpine réellement produit dans un navigateur.

## Prérequis

- Le serveur kreek doit déjà tourner en local avec `BYPASS_AUTH=true`
  (ex: `ENV=dev cargo run`, ou via `.env.dev`). Les tests ne le démarrent pas
  eux-mêmes — ils échouent immédiatement et clairement si rien ne répond sur
  `http://localhost:3210`.
- Le serveur doit servir le **jeu de démonstration** (`assets/references.example`),
  sur lequel les tests s'appuient : rosters Granitiers, Zéphyriens et
  Lanterniers. C'est le défaut de `config/default.toml`, donc `make dev`
  suffit — sauf si ton `.env.dev` surcharge `REFERENCES__DIR` vers un jeu de
  règles réel, auquel cas lance `make dev-demo`.
- La base doit contenir le seed de la suite : `make seed_e2e`. Il crée un space
  « Espace E2E », le coach `DevCoach` (`legacy_id=1`, celui que `BYPASS_AUTH`
  connecte) et onze autres coachs pour alimenter les sélecteurs. Entièrement
  synthétique et idempotent — rejouable après n'importe quel `make reset_db`.
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
| `E2E_SPACE_ID` | résolu par nom | Space ciblé par les tests — **toutes** les compétitions et équipes qu'ils créent y atterrissent. Par défaut, résolu au lancement via `GET /app/spaces` en cherchant l'espace nommé « Espace E2E » (créé par `make seed_e2e`). Ce n'est délibérément **pas** le premier espace venu : sur une base contenant de vraies données, ce raccourci polluerait un espace de production. Si l'espace dédié est absent, la suite s'arrête. Surcharger uniquement pour cibler un espace précis. |
| `E2E_COMPETITION_ID` / `E2E_SEASON_ID` | `E2E_SPACE_ID` | Utilisés seulement par `competition_rules_url` (la page accepte des IDs inexistants) |

**Si la résolution échoue** ("Aucun space_id trouvé") : la base ne contient pas le seed de la suite, ou l'utilisateur `legacy_id=1` n'appartient à aucun space. Lancer `make seed_e2e`, qui rend DevCoach membre du space par construction.

> Historiquement, la suite s'appuyait sur `make init_db WITH_SEED=1`, dont `seed_space_members.py` affecte 100 coachs **aléatoires** par espace — sans garantie que `legacy_id=1` en fasse partie, d'où des échecs intermittents au premier écran. `make seed_e2e` supprime cet aléa.

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

## Harnais visuel — `visual/`

Vérifie qu'une modification de CSS **ne change aucune valeur calculée**, ce
qu'exige la carte 341 avant de scoper les feuilles de style.

```bash
uv run python visual/releve.py avant     # avant de toucher au CSS
#  … modification …
uv run python visual/releve.py apres
uv run python visual/comparer.py avant apres
```

Le relevé couvre **43 pages × 2 largeurs**, soit les 49 feuilles de `pages/` et
`widgets/` chargées par l'application — couverture mesurée, pas supposée : une
page qui ne charge pas la feuille attendue est signalée, une page dont l'entité
manque en base aussi.

Il compare des **styles calculés** et non des captures d'écran. Un harnais par
images a été écrit d'abord puis abandonné : après trois corrections successives
il variait encore de 5 à 12 % d'un passage à l'autre sans qu'aucun CSS n'ait
changé. Les styles calculés sont déterministes ici — deux relevés consécutifs
donnent 0 écart sur 79 680 relevés — et un écart nomme l'élément et la
propriété au lieu de dire « cette page a changé ».

Les relevés vivent dans `visual/releves/`, hors dépôt : ils valent le temps
d'un lot, pas d'un commit.
