# La valeur d'équipe oublie les recrues d'un recrutement manuel

**Priorité : haute** — une valeur d'équipe fausse, que le prochain avant-match
lit pour calculer les coups de pouce
**Épic :** aucune — suite des phases manuelles (cartes 575 à 579)
**Dépend de :** rien
**Fichiers :** `src/app/players/domain/events.rs`,
`src/app/players/io/app_events/player_recruited_listener.rs`,
`src/app/players/context.rs`, `src/app/shared_kernel/app_events/players_app_events.rs`,
`src/app/teams/io/listeners/team_value_listener.rs`,
`tests/e2e/test_manual_phase_override.py`

## Le constat — mesuré le 2026-10-05

Une équipe `DEMO_GRANIT` prête à jouer ; un recrutement ouvert à la main ; un
Piétaille acheté (50 kPo) ; la validation.

```
avant              : valeur 550, 11 joueurs
après validation   : valeur 550, 12 joueurs   ← la recrue existe
5 secondes après   : valeur 550, 12 joueurs   ← la valeur ne la compte pas
événements : … PlayerRecruited, ManualPhaseClosed, TeamValueRecomputed
```

Le recalcul a lieu (`TeamValueRecomputed`), mais **avant** que `players` ait créé
la recrue, et rien ne le refait ensuite. La valeur reste fausse jusqu'au
prochain retour à « prête à jouer » — après le match suivant, dont l'avant-match
l'aura lue.

## La cause

`ManualPhaseClosed` part dans le même lot que les `PlayerRecruited`. Le listener
de valeur d'équipe recalcule aussitôt ; `players` crée la recrue de son côté, en
réaction à l'app event — et **n'annonce rien** quand c'est fait. `changes_squad`
ne relance le calcul que sur `InitialRosterCompleted`, `PlayerDismissed` et
`PlayerValueCustomised`.

Le parcours normal ne le voit pas : les renvois suivent le recrutement, et le
recalcul ne vient qu'à leur validation, recrues créées depuis longtemps.

Les deux autres sorties manuelles sont justes : la dépense de SPP précède le
recalcul ; les renvois relancent le calcul par `PlayerDismissed`.

## Le changement

La mécanique de `PlayerDismissed`, qui existe pour la course jumelle :

- `players` émet un événement de domaine `PlayerJoinedRoster { team_id,
  player_id }` sur son bus interne, **après** avoir écrit la recrue — non
  persisté, comme `InitialRosterCompleted`.
- **Les recrues seulement** (`RosterMembership::Active`), pas les journaliers
  alignés pendant la saisie d'un match : la valeur d'équipe ne se recalcule pas
  en cours de match, et ce n'est pas cette carte qui en décidera.
- Le publisher de `players` le convertit en app event `PlayerJoinedRoster`.
- `teams` l'ajoute à `changes_squad` : la valeur est recalculée une fois
  l'effectif réellement écrit, quel que soit le parcours.

## Tests

Unitaires : `to_app_event` produit l'app event ; `changes_squad` y réagit et
nomme l'équipe ; un journalier aligné n'émet rien.

E2E : après un recrutement manuel, la valeur d'équipe passe de 550 à 600.

## Terminé quand

Une recrue achetée pendant un recrutement ouvert à la main figure dans la valeur
d'équipe à la sortie de la phase, et la suite complète passe.
