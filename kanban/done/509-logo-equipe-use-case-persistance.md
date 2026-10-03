# Persister le changement de logo — event store + projection

**Priorité : moyenne**
**Dépend de :** carte 508 (méthode domaine `change_logo`)
**Fichiers :** `src/app/teams/use_cases/change_team_logo_use_case.rs` (nouveau),
`src/app/teams/io/repository/team_repository.rs`

## Objectif

Un use case qui charge l'agrégat, appelle `change_logo`, persiste l'événement
et met à jour la projection `team_proj.logo_url` dans la **même transaction**
(règle CLAUDE.md « Projections event sourcing » — pas de projection
désynchronisée possible).

## Plan

```rust
pub async fn execute(
    team_repository: &dyn TeamRepositoryPort,
    team_id: &str,
    logo_url: Option<CloudinaryImage>,
) -> Result<(), TeamUseCaseError> {
    let team = team_repository.find_by_id(team_id).await?
        .ok_or(TeamUseCaseError::NotFound)?;
    let event = team.change_logo(logo_url)?;
    team_repository.append_and_project(team_id, event).await?;
    Ok(())
}
```

(nom exact des méthodes de port à aligner sur ce qui existe déjà dans
`team_repository.rs` — vérifier comment `dismiss` ou une autre commande
persiste déjà son événement + projection, pour réutiliser le même point
d'entrée plutôt que d'en inventer un nouveau.)

## Vérification préalable obligatoire

Avant d'écrire, lire comment une mutation existante similaire (`dismiss`,
`approve_enrollment`) persiste déjà événement + projection dans
`team_repository.rs`, et suivre exactement ce chemin — ne pas réinventer un
mécanisme de transaction parallèle.

## Checklist

- [ ] Use case `change_team_logo_use_case::execute`
- [ ] Réutilise le mécanisme de persistance transactionnelle existant (pas de nouveau pattern)
- [ ] `team_proj.logo_url` mis à jour après exécution, y compris remis à `NULL` sur retrait — vérifié par test d'intégration sur vraie PgPool
- [ ] Erreur si équipe inconnue
- [ ] `make lint`, `make test`
