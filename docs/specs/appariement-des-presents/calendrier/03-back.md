# Calendrier · appariement des présents — architecture back

## Ce que l'investigation a montré

**Aucun port nouveau.** Le tirage a besoin des poules (`IGroupRepository`,
interne au BC), de l'historique des rencontres et du nombre de matchs joués
(les journées de la saison, internes au BC), et du coach de chaque équipe pour
R4 — `coach_id`, déjà dans `TeamInfoDto`, rendu par `ITeamInfoPort`.

**L'écriture d'une journée n'est pas atomique sur `main`.**
`generate_pairings.rs` appelle `save_pairing` rencontre par rencontre, une
transaction chacune : une panne à la troisième laisse une journée à moitié
appariée, et deux générations concurrentes se superposent. R11 exige au
contraire de constater « journée toujours vide » au moment d'écrire. La branche
`sondage-presence` l'a résolu (sa carte 509) par `save_pairings`, qui verrouille
la journée (`FOR UPDATE`) et écrit tout en une transaction.

**`generate_round_pairings` n'a qu'un appelant**, `generate_pairings.rs`, par
lequel passe aussi « Générer toutes les rencontres ». Ses autres occurrences
sont ses propres tests et un commentaire de `tests/e2e/test_invariant_journee.py`.

## Les fichiers

Tout vit dans le BC `competitions`. « Copié » : repris à l'identique depuis la
branche `sondage-presence` (règle 5), puis renommé en anglais (règle 18).

| Couche | Fichier | Rôle |
|---|---|---|
| Domaine | `domain/draw.rs` — copié de `tirage.rs` | l'algorithme : R4, R5, R8, départage au sort |
| Domaine | `domain/match_day.rs` — modifié | accepter ou refuser un tirage proposé (R10, R11) ; suppression de `generate_round_pairings` |
| Use case | `use_cases/admin/draw/draw_input.rs` — copié de `entree_du_tirage.rs` | l'entrée du tirage : historique, paires interdites, matchs joués |
| Use case | `use_cases/admin/draw/draw_candidates_service.rs` | lecture : équipes inscrites par poule, avec leur nombre de matchs |
| Use case | `use_cases/admin/draw/preview_draw_use_case.rs` | tirage par poule parmi les présents, **sans écrire** |
| Use case | `use_cases/admin/draw/confirm_draw_use_case.rs` | revérifie (R11), écrit d'un bloc, émet `PairingCreated` |
| Use case | `use_cases/admin/pairing_created.rs` — copié de `appariement_ecrit.rs` | l'émission de `PairingCreated`, partagée par la confirmation et la génération |
| Use case | `use_cases/admin/generate_pairings.rs` — modifié | nouveau tirage sur toutes les inscrites, écriture par `save_pairings` (R7, R12) |
| Dépôt | port et `match_day_repository.rs` : `save_pairings` — copié | écriture atomique sous verrou, avec son test d'intégration |
| Web | `io/web/admin/schedule_draw_widget.rs` | `GET …/schedule/draw` — l'étape « Présents » |
| Web | `io/web/admin/schedule_draw_actions.rs` | `POST …/draw/preview`, `POST …/draw/confirm` |
| Templates | `admin/widgets/schedule-draw-presence.html`, `schedule-draw-preview.html` | les deux étapes du widget |
| Templates | `admin/widgets/schedule-round-detail.html` — modifié | les deux boutons, sur journée vide seulement |
| Routes | `routes.rs`, `router.rs` | trois routes, sous `require_admin_access` et le contrôle « journée de la saison » existant |
| CSS | `assets/static/css/widgets/schedule-draw.css`, `src/web/css_bundle.rs` | la feuille du widget |

## Qui décide quoi

| Question | Où |
|---|---|
| Quelles rencontres tirer ? | domaine, `draw.rs` |
| Ce tirage proposé est-il acceptable ? | domaine, agrégat **`MatchDay`** |
| Quelles données charger, poule par poule ? | use cases |
| Écrire, émettre | use cases |
| Quel fragment rendre, quel toast | handlers, à partir de view models — jamais de `TeamInfoDto` |

`MatchDay` vérifie qu'il est vide, que chaque équipe n'y paraît qu'une fois,
qu'aucune rencontre n'oppose deux équipes d'un même coach, et que chaque
rencontre réunit deux équipes de la même poule. C'est la journée qui reçoit les
rencontres : la règle lui appartient. La branche la posait sur l'agrégat de la
campagne, qui n'est pas repris.

## Événements

Aucun nouveau. `PairingCreated` est déjà publié vers les autres BCs, et
`match_report` en tire le rapport de match.

## Ce qui n'est pas repris de la branche

L'agrégat `PresenceSurvey`, la campagne, les e-mails, les jetons, la réparation
et l'annulation d'un tirage.

## Règles métier ajoutées à cette étape

| # | Règle |
|---|---|
| R13 | Une rencontre écrite par la confirmation ou la génération est une rencontre ordinaire du Calendrier : elle se supprime, se déplace, se complète comme aujourd'hui. |
