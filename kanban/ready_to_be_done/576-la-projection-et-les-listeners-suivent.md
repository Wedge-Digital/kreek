# La projection et les listeners suivent les phases manuelles

**Priorité : moyenne**
**Épic :** aucune — fonction « phases manuelles », cartes 575 à 579
**Dépend de :** 575
**Spec :** `docs/specs/phases-manuelles/bandeau-fiche-equipe/07-integration.md` (points 2 à 5)
**Fichiers :** `src/app/teams/io/repository/team_repository.rs`,
`src/app/teams/io/listeners/team_value_listener.rs`,
`src/app/teams/io/listeners/phase_basket_purge_listener.rs`,
`src/infrastructure/match_report/ref_team_data_adapter.rs`

## L'objectif

Une phase ouverte à la main se lit dans `team_proj`, et sa fermeture recalcule
la valeur d'équipe et purge les paniers, comme une sortie d'après-match.

## Le changement

- `team_proj` : le `_ => {}` disparaît ; les 34 événements sont classés. Bras
  neufs : les trois ouvertures, `ManualPhaseClosed`, et — non émis aujourd'hui —
  `TeamRenamed`, `OffSeasonStarted`, `RetirementPhaseValidated`,
  `OffSeasonCompleted`. `LogoChanged` reste à la PR #11, dans le groupe sans
  effet avec un commentaire.
- Les deux listeners appellent `returns_to_ready_to_play()` ; leurs copies de la
  liste disparaissent.
- `is_team_in_player_improvement` ne répond oui qu'à une entrée `PostMatch`.

## Tests

Dépôt : chaque ouverture pose la bonne `game_phase` en projection,
`ManualPhaseClosed` pose `ReadyToPlay`, les quatre bras d'avance écrivent leur
colonne. Listeners : `ManualPhaseClosed` déclenche le recalcul et la purge.
Adapter : une phase de dépense manuelle n'est pas « en amélioration » pour la
correction.

## Terminé quand

`team_proj` et la fiche disent la même phase pendant et après une phase
manuelle, et `make test` passe.
