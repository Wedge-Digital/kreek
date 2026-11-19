# BC `teams` — Use cases de validation de phase (amélioration / recrutement / renvois)

**Priorité : haute**
**Dépend de :** `169-teams-domain-dismissals-target-match-report-id.md` (pour que la validation des renvois cible bien `ReadyToPlay`)
**Contexte :** `teams` — actions coach, couche use case + IO web

## Objectif

Câbler les 3 actions de transition de phase du bandeau d'état : "Évolutions
terminées", "Terminer les achats", "Valider les renvois". Les méthodes domaine
(`validate_improvement_phase`, `validate_recruitment_phase`,
`validate_dismissals_phase`) existent déjà — cette carte n'ajoute que
l'orchestration use case + les routes HTTP. Spec complète :
`docs/specs/team-state-management/team-detail/02-07-conception.md` (Phases 4-5, 7).

---

## Conception

### Commandes (`use_cases/commands.rs`)

```rust
pub struct ValidateImprovementPhaseCommand { pub team_id: TeamId }
pub struct ValidateRecruitmentPhaseCommand { pub team_id: TeamId }
pub struct ValidateDismissalsPhaseCommand  { pub team_id: TeamId }
```

### 3 use cases (même forme que `use_cases/dismiss_team.rs`)

```rust
// validate_improvement_phase_use_case.rs
pub enum ValidateImprovementPhaseError { TeamNotFound, Domain(DomainError), Repository(RepositoryError) }

pub async fn execute(
    cmd: ValidateImprovementPhaseCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ValidateImprovementPhaseError> {
    let team = team_repo.find_by_id(&cmd.team_id.to_string()).await
        .map_err(ValidateImprovementPhaseError::Repository)?
        .ok_or(ValidateImprovementPhaseError::TeamNotFound)?;
    let event = team.validate_improvement_phase().map_err(ValidateImprovementPhaseError::Domain)?;
    team_repo.append(&cmd.team_id.to_string(), &event, team.version).await
        .map_err(ValidateImprovementPhaseError::Repository)?;
    Ok(())
}
```

Fichiers : `validate_improvement_phase_use_case.rs`, `validate_recruitment_phase_use_case.rs`,
`validate_dismissals_phase_use_case.rs` — chacun appelle sa méthode domaine
correspondante. Aucune logique métier dans ces fichiers.

### Routes (`routes.rs`)

```
POST /app/{space_id}/teams/{team_id}/validate-improvement-phase
POST /app/{space_id}/teams/{team_id}/validate-recruitment-phase
POST /app/{space_id}/teams/{team_id}/validate-dismissals-phase
```

### Handlers (`io/web/team_detail.rs` ou fichier dédié `validate_phase_actions.rs`)

Un handler par route : extrait `team_id` du path (valide via `TeamId::try_new`),
construit la commande, appelle le use case, retourne `HX-Refresh: true` en
succès (recharge la page — le changement de phase impacte aussi le badge
d'en-tête), fragment d'erreur HTMX standard sinon (`AppError`).

---

## Checklist

- [ ] 3 commandes dans `commands.rs`
- [ ] 3 use cases (`validate_improvement_phase_use_case.rs`, `validate_recruitment_phase_use_case.rs`, `validate_dismissals_phase_use_case.rs`)
- [ ] 3 routes ajoutées dans `routes.rs` + `router.rs`
- [ ] 3 handlers HTTP (POST, réponse `HX-Refresh: true` en succès)
- [ ] Test : appel hors phase attendue → erreur domaine propagée proprement (pas de panic)
