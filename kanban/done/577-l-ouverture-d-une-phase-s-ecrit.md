# L'ouverture d'une phase s'écrit

**Priorité : moyenne**
**Épic :** aucune — fonction « phases manuelles », cartes 575 à 579
**Dépend de :** 575, 576
**Spec :** `04-dtos.md`, `05-use-cases.md`
**Fichiers :** `src/app/teams/use_cases/commands.rs`,
`src/app/teams/use_cases/open_phase_override_use_case.rs` *(nouveau)*,
`src/app/teams/use_cases/mod.rs`

## L'objectif

Un use case enregistre l'ouverture d'une phase manuelle : il charge l'équipe,
demande l'événement à l'agrégat, l'enregistre.

## Le changement

- `OpenPhaseOverrideCommand { team_id, phase: OverridablePhase, reason:
  Option<OverrideReason>, admin_id, admin_name }`.
- `open_phase_override_use_case::execute`, instrumenté ;
  `OpenPhaseOverrideError { TeamNotFound, Domain, Repository }`.
- Pas de contrôle de droit (le contrôleur), pas d'app event (groupe muet).

## Tests

Une ouverture s'enregistre ; une équipe qui n'est pas prête à jouer est refusée
sans rien écrire ; une équipe introuvable rend `TeamNotFound`.

## Terminé quand

Une ouverture est en base, la projection suit, et `make test` passe.
