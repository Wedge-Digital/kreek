# Phase 2 — Architecture front — team-detail

## Périmètre

Édition inline de l'effectif (nom, numéro de maillot, ordre d'affichage)
depuis le tableau joueurs de la page de détail d'équipe, déclenchée depuis le
bandeau d'état « Prête à jouer ». Seuls les joueurs `membership = 'Active'`
sont concernés (mêmes joueurs que ceux déjà listés par le widget aujourd'hui —
`find_by_team_id` filtre déjà sur `Active`, aucun changement de portée côté
requête).

## Widgets et communication

Un seul widget touché : `players-widget` (BC `players`,
`player_table.rs` / `player-table-fragment.html`, déjà existant). Le bandeau
d'état n'est pas un widget — il est rendu inline dans la page hôte du BC
`teams` (`teams-team-detail.html`). Déclencheur et cible étant dans deux BCs
différents, toute la coordination passe par des événements DOM sur `body`
(Règle 2 CLAUDE.md) — jamais d'accès direct au DOM de l'autre, jamais de
`hx-include`/`hx-target` traversant la frontière.

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| players-widget | players | `GET .../players/table` (existant, inchangé) | `load` (page) ; `rosterEditRequested from:body` (bascule locale, pas de round-trip) | `rosterEditValidityChanged`, `rosterEditSaved`, `rosterEditSaveFailed` | lecture ↔ édition |
| bandeau d'état (host, pas un widget) | teams | — | — | `rosterEditRequested`, `rosterEditCancelRequested`, `rosterEditSaveRequested` | déclencheur |

### Événements

- **`rosterEditRequested`** — pas de payload. Émis par le clic sur « Modifier
  l'effectif » (bandeau). Écouté par le players-widget : bascule de classe
  CSS (mode édition), aucune requête serveur.
- **`rosterEditCancelRequested`** — pas de payload. Émis par « Annuler »
  (bandeau). Écouté par le players-widget : restaure le snapshot pris à
  l'entrée en édition, ressort du mode édition. Aucune requête serveur.
- **`rosterEditSaveRequested`** — pas de payload. Émis par « Enregistrer »
  (bandeau). Le `<form>` du players-widget (enveloppe `#roster-tbody`) écoute
  via `hx-trigger="rosterEditSaveRequested from:body"` et déclenche son propre
  POST.
- **`rosterEditValidityChanged { valid: bool }`** — émis par le players-widget
  à chaque saisie sur un champ numéro (JS, vérif de doublon locale). Écouté
  par le bandeau pour activer/désactiver « Enregistrer ».
- **`rosterEditSaved`** — émis côté serveur via header `HX-Trigger` sur la
  réponse 200 du POST. Écouté par le bandeau : rebascule ses boutons
  (Enregistrer/Annuler → Modifier l'effectif).
- **`rosterEditSaveFailed { message: string }`** — émis côté serveur via
  `HX-Trigger` si le POST échoue une validation métier (ex. doublon de numéro
  détecté serveur malgré la vérif front — cas de concurrence entre deux
  onglets/coachs). Écouté par le players-widget uniquement : affiche
  `message`, **reste en mode édition** (pas de perte de saisie, l'utilisateur
  corrige et retente).

### Actions

- **`POST .../players/roster`** — formulaire porté par le players-widget
  lui-même (pas par le bandeau). Champs soumis par ligne :
  `player_id[]`, `personal_name[]`, `jersey[]` — dans l'ordre visuel des
  lignes (le glisser-déposer réordonne le DOM). Pas de champ d'ordre dédié :
  le use case assigne `display_order = index` dans le tableau reçu.
  Réponse : fragment `players-widget` à jour, mode lecture, header
  `HX-Trigger: rosterEditSaved` en succès.

## Front vs back

- **Front seul** : bascule lecture/édition (classe CSS), glisser-déposer
  (réordonne le DOM, aucune requête), validation de doublon de numéro en
  direct (JS, feedback immédiat sans aller-retour serveur).
- **Back** : uniquement au clic Enregistrer — un seul POST batch. Le use case
  ne persiste et n'émet un événement domaine que pour les champs réellement
  modifiés par joueur (pas de `PlayerRenamed` si le nom n'a pas changé, etc.).

## États

- **Lecture** — existant, maquette validée (Phase 1).
- **Édition** — maquette validée (Phase 1).
- **Soumission en cours** — pas de maquette dédiée : `hx-indicator` du
  formulaire cible le bouton « Enregistrer » du bandeau (sélecteur CSS
  cross-DOM, htmx le permet nativement) → libellé "Enregistrement…" et
  `disabled` le temps de la requête.
- **Erreur de sauvegarde** — pas de maquette dédiée (décision explicite : cas
  de concurrence jugé rare). Bandeau d'erreur inline discret au-dessus du
  tableau, dans le players-widget : `Impossible d'enregistrer : {message}`.
  Le mode édition n'est pas quitté.

## Widgets existantes réutilisées

Aucune nouvelle widget : extension de `players-widget` existant. Pas de
widget réutilisable trouvé ailleurs pour renommage/renumérotation/ordre
(vérifié : aucune occurrence de `PlayerRenamed`/`JerseyChanged`/`display_order`
dans le code actuel).

## Règles métier identifiées à cette étape

- Seuls les joueurs `membership = 'Active'` sont éditables (confirmé).
- Plage valide du numéro de maillot : `1..16` reprise de la maquette —
  **provisoire, à confirmer en Phase 6** (à vérifier contre une éventuelle
  règle roster réelle plutôt qu'une valeur codée en dur dans le HTML).
- Nom du joueur : pas de contrainte de longueur/caractères définie —
  **à trancher en Phase 6**.
- Unicité du numéro de maillot : vérifiée front (feedback immédiat) et back
  (garde-fou, cas de concurrence) — la vérité métier est côté back.
