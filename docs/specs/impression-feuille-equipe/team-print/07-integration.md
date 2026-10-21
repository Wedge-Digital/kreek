# Page « Imprimer » — Phase 7 : intégration

Entrée : `06-domaine.md`. Conception, pas implémentation.

## Persistance

**Aucune table, aucune migration, aucune écriture.** Lectures seulement :

| BC | Lecture | Nouvelle ? |
|---|---|---|
| teams | `ITeamRepository::find_by_id` | existante |
| teams | `ISquadPort::find_squad`, `IRosterCatalogPort::find_catalog` | existantes |
| players | `IPlayerProjectionRepository::find_alive_by_team_id` (avec `membership`) | existante |
| competitions | `ITeamInfoPort::find_team_enrollment`, `IMatchDayRepository::list_team_matches` | existantes |
| match_report | `IPlayerDataPort::list_alignable_players` | **nouvelle** — adapter sur `find_alive_by_team_id`, filtre `presence` alignable |
| match_report | `ITeamDataPort::find_team_season` | **nouvelle** — `team.season_id` |
| match_report | `IReferenceCatalogPort` (4 méthodes) | **nouveau port** — `src/infrastructure/match_report/reference_catalog_adapter.rs` |
| references | `match_reminders_fr.json` | **nouveau fichier optionnel** — lecture dans `in_memory_reference_repository.rs` |

## Événements

Aucun : ni domain event, ni app event, ni listener. Les axes 5, 12 et 18 ne
sont pas concernés.

## Sécurité

`ISpaceOwnership` pour le paramètre `other_team_id` :
`src/infrastructure/teams/space_ownership.rs`, même résolution que `team_id`
(`team_proj`), enregistré dans le `vec!` de `main.rs`. Toute route à seconde
équipe la déclare dans son chemin.

## Handlers

Tous : `Result<impl IntoResponse, AppError>`, extracteurs `Path`, `Query`,
`State<AppState>`, `AuthSession`.

**Droit d'imprimer (R16)** : tout utilisateur connecté (`require_auth`),
comme la fiche d'équipe, lisible par tous (`team_detail.rs:613-616`). Aucun
handler ne prend `SpacePermissions`. L'appartenance des équipes à l'espace,
elle, est contrôlée partout par `space_scope`.

| BC | Handler | Path | Query | Rend |
|---|---|---|---|---|
| teams | `get_team_print_page` | `(space_id, team_id)` | — | page complète (`app-layout`) ou page d'explication (R1) |
| teams | `get_print_preview` | `(space_id, team_id)` | `PrintOptionsQuery` | fragment |
| teams | `get_print_preview_with` | `(space_id, team_id, other_team_id)` | `PrintOptionsQuery` | fragment |
| teams | `get_team_sheet_head` | `(space_id, team_id)` | `role`, `treasury` | fragment |
| players | `get_player_print_table` | `(space_id, team_id)` | `unavailable`, `blank_rows`, `density` | fragment |
| players | `get_skills_annex` / `…_with` | `(space_id, team_id[, other_team_id])` | `orientation`, `density` | fragment, 1 à n feuilles |
| competitions | `get_next_opponent_option` | `(space_id, team_id)` | — | fragment |
| match_report | `get_inducements_print` / `…_with` | `(space_id, team_id[, other_team_id])` | `inducements`, `star_players`, `orientation`, `density` | fragment, 1 à n feuilles |
| match_report | `get_match_sheet` / `…_with` | `(space_id, team_id[, other_team_id])` | `home`, `round_id` | fragment, 1 feuille |

Le bouton de la fiche (`BannerCtaVm::Print`) devient un lien vers
`get_team_print_page`, construit par `Routes` de teams. Même condition
d'affichage qu'aujourd'hui : équipe prête à jouer (R1), pour tout visiteur —
c'est le seul CTA que `BannerVm::from_domain` garde quand `peut_editer` est
faux, et il le reste.

## Templates et CSS

| Fichier | BC | Contenu |
|---|---|---|
| `teams-print-page.html` | teams | options, cascade `kreek-select`, zone d'aperçu, barre d'actions, script de la page |
| `teams-print-unavailable.html` | teams | explication R1, lien vers la fiche |
| `widgets/print-preview.html` | teams | feuilles, emplacements des fragments |
| `widgets/print-sheet-head.html` | teams | en-tête de fiche |
| `widgets/print-player-table.html` | players | tableau en lecture seule |
| `widgets/print-skills-annex.html` | players | annexe |
| `widgets/print-next-opponent.html` | competitions | option du prochain adversaire |
| `widgets/print-inducements.html` | match_report | coups de pouce, star players |
| `widgets/print-match-sheet.html` | match_report | feuille de match |

- `app-layout.html` : un `{% block body_class %}{% endblock %}` sur `<body>`,
  vide par défaut ; la page d'impression y met `team-print-page`.
- CSS : `pages/team-print.css` (page, aperçu, feuilles, `@media print`, pages
  nommées `a4-portrait` / `a4-landscape`) et une feuille par fragment sous
  `widgets/`, chacune nommée d'après sa racine. Toutes inscrites dans
  `FEUILLES_APP` (`src/web/css_bundle.rs`, axe 14).
- Aucun `style="…"`, aucun `<link>` dans les gabarits, aucun `<select>` natif.
- Le script de la page : `document.currentScript.previousElementSibling`,
  fonctions ≤ 20 lignes — réduction à l'écran, suivi du chargement,
  densité automatique, `printPreviewReady`, `window.print()`.

## Corpus

- `assets/references.example/match_reminders_fr.json` : version fictive
  (`mvp`, `fans`, `weather`).
- `docs/reference-data-schema.md` : section du nouveau fichier ; correction
  des douze fichiers obligatoires et de `spp_rules.json`.

## Tests E2E prévus

Fichiers `tests/e2e/test_team_print_*.py`, ajoutés à `tests/impact-map.toml`
dans le même commit (axe 8). Le clic sur du contenu injecté passe par
`cliquer_quand_cable`.

| # | Scénario | PR |
|---|---|---|
| E1 | Le bouton « Imprimer » d'une équipe prête à jouer mène à la page ; il est absent en recrutement ; l'URL directe d'une équipe non prête affiche l'explication | 1 |
| E1b | Un coach connecté qui ne possède pas l'équipe voit le bouton et imprime ; un visiteur non connecté est renvoyé à la connexion | 1 |
| E2 | Cocher et décocher une option recharge l'aperçu ; le nombre de pages suit | 1 |
| E3 | Le prochain adversaire est proposé avec sa journée ; sans match à venir, l'option est grisée avec son motif | 1 |
| E4 | La cascade compétition → saison → équipe ajoute la fiche de l'autre équipe | 1 |
| E5 | Une seconde équipe d'un autre espace dans l'URL d'un fragment répond 404 | 1 |
| E6 | `page.pdf()` (émulation impression) produit exactement autant de pages que l'aperçu a de feuilles, sans le texte des options | 1 |
| E7 | « Imprimer » reste désactivé jusqu'à `printPreviewReady`, puis appelle `window.print()` (doublé dans le test) | 1 |
| E8 | Une feuille trop pleine passe en densité compacte ; rien n'est coupé | 1 |
| E9 | Annexe des compétences : chaque compétence une seule fois, avec sa description | 2 |
| E10 | Coups de pouce : prix par roster, « — » pour un coup de pouce réservé ; star players filtrés par roster | 2 |
| E11 | Feuille de match : favori et petite monnaie conformes aux VE ; journaliers marqués ; barème SPP de chaque roster ; rappels et météos du corpus | 2 |
