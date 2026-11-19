# Le domaine sait ouvrir une phase manuelle

**Priorité : moyenne**
**Épic :** aucune — fonction « phases manuelles », cartes 575 à 579
**Dépend de :** 570 (livrée)
**Spec :** `docs/specs/phases-manuelles/bandeau-fiche-equipe/06-domaine.md`
**Fichiers :** `src/app/teams/domain/team.rs`, `src/app/teams/domain/value_objects.rs`,
`src/app/teams/io/app_events/app_event_publisher.rs` (groupe muet, exigé par la
compilation)

## L'objectif

L'agrégat `Team` sait ouvrir, depuis « prête à jouer », la dépense des SPP, le
recrutement ou les renvois, et ramène l'équipe à « prête à jouer » à la sortie
d'une phase ouverte ainsi — sans enchaîner, sans erreurs coûteuses.

## Le changement

- `PhaseEntry { PostMatch, Override }`, `PostMatch` par défaut : champ dérivé
  `phase_entry`, reconstruit au rejeu.
- `OverridablePhase { PlayerImprovement, Recruitment, Dismissals }` : `ALL`,
  `parse`, `as_str`, `game_phase`.
- `OverrideReason` : `trim`, 200 caractères au plus, `TEXTE_SAISI`.
- Quatre événements : `ManualImprovementPhaseOpened`,
  `ManualRecruitmentPhaseOpened`, `ManualDismissalsPhaseOpened` (`admin_id`,
  `admin_name`, `reason`), `ManualPhaseClosed { phase }` ; leurs bras de
  `type_name()`, de `apply()` et du groupe muet de `to_app_event()`.
- `open_phase_override` : `expect_phase(ReadyToPlay)`, puis l'événement de la
  phase choisie.
- `apply()` : les ouvertures posent la phase et `Override` ; `ManualPhaseClosed`
  pose `ReadyToPlay` et `PostMatch` ; `PostMatchSequenceStarted` remet
  `PostMatch`.
- Les trois sorties rendent `ManualPhaseClosed` sur une entrée `Override` ; les
  renvois sans erreurs coûteuses, quelle que soit la trésorerie.
- `revert_post_match_sequence` refuse une entrée `Override`
  (`NoPostMatchToRevert`).
- `TeamDomainEvent::returns_to_ready_to_play()` : `match` exhaustif, sans
  joker.

## Tests

Ceux du point 8 de la spec de domaine.

## Terminé quand

`cargo test` couvre chacune des règles de `06-domaine.md`, et `make check-arch`
passe.
