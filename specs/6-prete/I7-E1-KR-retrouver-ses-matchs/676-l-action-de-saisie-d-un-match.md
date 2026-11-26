---
id: 676-l-action-de-saisie-d-un-match
parent: I7-E1-KR
ticket: I7-E1-KR-T2
status: approved
---

# L'action de saisie d'un match

**Epic :** I7-E1-KR · **Besoin :** I7-B3 · **Dépend de :** —

## Le constat

- Chaque appariement a son brouillon (`pairing_created_listener`, origine `Pairing`) ;
  `match_report_proj` porte `pairing_id`, `phase`, `entry_mode`, `started_by`
  (migrations `20260624000004`, `20261101000001`). Brouillon non commencé :
  `phase = 'Draft'` et `started_by IS NULL`.
- Les règles d'accès sont dans `use_cases/match_report_access_service.rs` :
  `EntryScope{entry_mode, started_by}`, `is_authorized`, `can_write_report` (en
  « Je saisis tout », le seul coach qui a commencé, sauf s'il n'entraîne aucune des
  deux équipes ; en « Chacun son équipe », les deux coachs).
- Le port de dépôt sait `find_entry_mode`, `find_id_by_pairing`,
  `find_phases_by_pairings` — **aucune requête ne rend, pour une liste
  d'appariements, l'état de saisie** (phase, mode, auteur).
- Les routes existent : `from_pairing` mène à « Qui saisit ? » ou à l'étape en cours ;
  `edit_match_report` aussi (la garde envoie l'autre coach en « Je saisis tout » vers
  `live`, et en « Chacun son équipe » chacun à son camp) ; `live` est la lecture seule.

## Ce qui change

- **Une décision du domaine**, l'action de saisie d'un match pour un coach :

  | État | Action |
  |---|---|
  | brouillon non commencé | **Commencer** |
  | commencé, et `can_write_report` vrai pour ce coach (il a commencé en « Je saisis tout », ou « Chacun son équipe ») | **Continuer** |
  | commencé en « Je saisis tout » par l'autre | **Suivre** |
  | publié, annulé, ou coach étranger au match | aucune |

  La règle réutilise `can_write_report` : pas de seconde règle d'écriture.
- **Une requête par lot** sur `match_report_proj` : pour une liste d'appariements, le
  rapport non annulé de chacun avec sa phase, son mode, son auteur.
- **Un service de lecture public** du BC (`use_cases/`, `arch:no-instrument`) :
  coach + espace + appariements → pour chacun, l'action et l'identifiant du rapport.
  C'est lui que l'adapter de `competitions` appellera en 677 ; le BC ne connaît pas
  son consommateur et ne construit pas d'URL d'un autre BC.

Pas de port côté `competitions` ni d'adapter ici : 677 les pose.

## Fichiers touchés

`match_report/domain/` (l'action), `match_report/domain/match_report_repository_port.rs`,
`match_report/io/repository/match_report_repository.rs` et son SQL,
`match_report/use_cases/<service neuf>.rs` ; les doublures du port de dépôt.

## Règles et invariants

- `match_report` décide seul de l'action ; personne d'autre ne relit sa projection.
- Un coach qui n'entraîne aucune des deux équipes n'obtient aucune action (un admin
  ne voit que ses équipes : I7-B1).
- Lecture seule.

## Tests

Unitaires : chaque ligne du tableau, dont « Chacun son équipe » commencé par
l'adversaire (Continuer) et « Je saisis tout » commencé par l'adversaire (Suivre) ;
la requête par lot sur une vraie base (appariement sans rapport, rapport annulé
exclu).

## Conséquences sur les données de production

Aucune : lecture seule. Les comptes rendus en cours à la livraison donnent
« Continuer » ou « Suivre » selon leur mode et leur auteur.

## Terminé quand

Le service rend la bonne action pour chacun des quatre états, démontré par les
tests unitaires et d'intégration.
