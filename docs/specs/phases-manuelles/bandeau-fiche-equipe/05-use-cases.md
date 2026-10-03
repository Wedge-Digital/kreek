# Bandeau de la fiche équipe · Phase 5 : use cases

## Le seul use case neuf : `open_phase_override_use_case.rs`

La forme de `validate_improvement_phase_use_case`, le plus simple de ses
voisins.

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: OpenPhaseOverrideCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), OpenPhaseOverrideError> {
    let team = team_repo.find_by_id(&cmd.team_id.to_string()).await
        .map_err(OpenPhaseOverrideError::Repository)?
        .ok_or(OpenPhaseOverrideError::TeamNotFound)?;

    let event = team
        .open_phase_override(cmd.phase, cmd.admin_id, cmd.admin_name, cmd.reason)
        .map_err(OpenPhaseOverrideError::Domain)?;

    team_repo.append(&cmd.team_id.to_string(), &event, team.version).await
        .map_err(OpenPhaseOverrideError::Repository)?;
    Ok(())
}
```

- **Il ne décide rien.** « Uniquement depuis prête à jouer » et le choix de
  l'événement d'ouverture appartiennent à l'agrégat (phase 6). Le use case
  charge, demande, persiste.
- **Pas de contrôle de droit ici** : le contrôleur a appliqué
  `require_team_admin`, comme pour l'ajustement de trésorerie.
- **Un événement, une transaction** : `append` écrit l'événement et la projection
  `team_proj` ensemble (règle des projections).
- **Pas d'app event** : `players` lit la phase en direct ; l'événement rejoint le
  groupe muet du publisher.
- **Instrumenté** ; la commande ne porte aucun secret.

## Les trois use cases de sortie : inchangés

| Use case | Aujourd'hui | Avec une phase manuelle |
|---|---|---|
| `validate_improvement_phase` | appelle `team.validate_improvement_phase()` | l'agrégat rend `ManualPhaseClosed` au lieu de `PlayerImprovementPhaseValidated` |
| `validate_recruitment_phase` | applique le panier, appelle `team.validate_recruitment_phase()`, supprime le panier | même enchaînement ; le dernier événement du lot devient `ManualPhaseClosed`, le panier est supprimé comme avant |
| `validate_dismissals_phase` | applique les renvois, appelle `team.validate_dismissals_phase()` | jamais de `CostlyMistakesPhaseStarted` pour une phase manuelle : `ValidateDismissalsOutcome::depuis_le_lot` rend `PreteAJouer`, le contrôleur ramène à la fiche |

**Les journaliers, à la sortie d'un recrutement manuel.** Un recrutement normal
se clôt par l'app event `RecruitmentPhaseValidated`, sur lequel `players` retire
les journaliers non embauchés. Un recrutement manuel se clôt par
`ManualPhaseClosed`, qui ne sort pas du BC. C'est juste : un journalier
n'existe qu'entre un match et la fin de son recrutement, donc une équipe prête
à jouer n'en a aucun. Un test de la phase 6 le vérifie.

## La sortie n'a rien à orchestrer

Le recalcul de la valeur d'équipe et la purge des paniers ne sont pas des use
cases : ce sont les deux listeners, qui réagissent à `ManualPhaseClosed` par
`returns_to_ready_to_play()`.

## Règles métier

Question posée le 2026-10-03 : la phase 5 n'en ajoute aucune. Elle organise des
appels ; les règles sont au domaine.
