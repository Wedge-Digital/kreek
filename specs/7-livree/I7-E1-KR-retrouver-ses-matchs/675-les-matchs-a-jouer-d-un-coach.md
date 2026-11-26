---
id: 675-les-matchs-a-jouer-d-un-coach
parent: I7-E1-KR
ticket: I7-E1-KR-T1
status: done
---

# Les matchs à jouer d'un coach

**Epic :** I7-E1-KR · **Besoins :** I7-B1, I7-B2 · **Dépend de :** —

## Le constat

- `list_team_matches(season_id, team_id)` (`competitions/domain/match_day_repository_port.rs:195`,
  SQL `io/repository/sql/match_days/list_team_matches.sql`) rend les rencontres d'une
  équipe, en cours d'abord puis à venir par `round_position`, sans date.
- Le prochain match existe déjà : `domain/next_opponent.rs` (`next_opponent`,
  `upcoming_opponents`, `PairingStatus::from_projection`) et
  `use_cases/next_opponent_service.rs` (`find`), servis à la page d'impression.
- La saison d'une équipe se lit par `ITeamInfoPort::find_team_enrollment`
  (`competitions/ports.rs:34`) — jamais depuis l'URL.
- **Rien ne donne les équipes d'un coach** à `competitions`. `teams` les connaît :
  `ITeamRepository::find_by_coach_and_space(coach_id, space_id)` (`teams/ports.rs:355`),
  avec leur statut.
- Le code ne connaît pas de saison « en cours » (décision : équipe inscrite, sans
  filtre de saison).

## Ce qui change

- **Port `ITeamInfoPort`** : une méthode qui rend les équipes **inscrites** d'un coach
  dans un espace (identifiant, nom…), implémentée dans
  `infrastructure/competitions/team_info_adapter.rs` à partir de
  `find_by_coach_and_space`, filtrée sur le statut inscrit.
- **Domaine** (`competitions/domain/`, module neuf, sur le modèle de `next_opponent`) :
  à partir des rencontres de chaque équipe, la liste des **matchs à jouer** (ni joués
  ni annulés) :
  - **sans doublon** : une rencontre entre deux équipes du coach n'y figure qu'une fois ;
  - **le prochain match** de chaque équipe marqué — la première rencontre non jouée,
    en cours d'abord, quelle que soit sa date ; une rencontre est « prochaine » si elle
    l'est pour l'une ou l'autre des équipes du coach ;
  - **triée** : en cours d'abord, puis par date limite de la journée
    (`round_date_end`) croissante, les rencontres sans date en dernier, à égalité par
    `round_position`.
- **Service** (`use_cases/`, `arch:no-instrument` — lecture) : coach + espace →
  équipes inscrites → saison de chacune → `list_team_matches` → la liste ci-dessus.
  Une équipe sans inscription lisible est ignorée, et journalisée.

Pas encore de page ni de route : 677 les pose.

## Fichiers touchés

`competitions/ports.rs`, `competitions/domain/<module neuf>.rs`,
`competitions/use_cases/<service neuf>.rs`,
`infrastructure/competitions/team_info_adapter.rs` ; les doublures de
`ITeamInfoPort` dans les tests existants.

## Règles et invariants

- `competitions` ne lit aucune table de `teams` : il passe par le port.
- Le prochain match d'une équipe est le même que celui de `next_opponent` — même
  règle, pas une seconde.
- Lecture seule.

## Tests

Unitaires : le domaine (doublon, prochain match par équipe, match lointain qui reste
le prochain, tri en cours / daté / sans date, match joué exclu), le service avec des
doublures (équipe non inscrite ignorée), l'adapter (filtre sur l'inscription).

## Conséquences sur les données de production

Aucune : lecture seule.

## Terminé quand

Le service rend, pour un coach à deux équipes inscrites, leurs matchs à jouer triés,
sans doublon, le prochain de chacune marqué — démontré par les tests unitaires.
