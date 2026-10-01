# Panneau d'ajustement · Phase 5 : use case

**Entrée** : `04-dtos.md` validé.

## Ce que le use case ne fait pas : émettre

Dans `teams`, **c'est le dépôt qui émet**, après le commit
(`team_repository.rs:400-408`), et le code dit pourquoi :

> « Publiés après le commit, dans l'ordre, et depuis le repository plutôt que
> depuis chaque use case : deux des quatre chemins vers `ReadyToPlay` passent
> par des listeners. C'est le seul point qui les couvre tous. Déviation assumée
> du patron de `players` et `match_report`. »

Le use case appelle `append`, et l'émission suit toute seule. La phase 3
affirmait le contraire ; elle est corrigée.

## Le fichier

`teams/use_cases/adjust_treasury_use_case.rs`

```rust
#[derive(Debug)]
pub enum AdjustTreasuryError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: AdjustTreasuryCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), AdjustTreasuryError> {
    let team_id = cmd.team_id.to_string();
    let team = team_repo
        .find_by_id(&team_id)
        .await
        .map_err(AdjustTreasuryError::Repository)?
        .ok_or(AdjustTreasuryError::TeamNotFound)?;

    let event = team
        .adjust_treasury(cmd.direction, cmd.amount, cmd.note, cmd.admin_id, cmd.admin_name)
        .map_err(AdjustTreasuryError::Domain)?;

    team_repo
        .append(&team_id, &event, team.version)
        .await
        .map_err(AdjustTreasuryError::Repository)?;

    Ok(())
}
```

C'est la forme d'`apply_costly_mistakes_use_case`, moins le dé et moins le
compte rendu. L'instrumentation est obligatoire — axe 11 — et `skip_all` est
indispensable : sans lui, l'attribut tenterait d'enregistrer le dépôt, qui
n'implémente pas `Debug`.

## Trois choix

### Il rend `()`, pas le nouveau solde

`apply_costly_mistakes` rend un `CostlyMistakesOutcome`, « de quoi l'annoncer au
coach sans relire l'agrégat ». Ici on relit de toute façon : la réponse est la
page entière, et le relevé va chercher le solde dans le grand livre. Rendre un
solde créerait une seconde source pour un chiffre qui en a déjà une — exactement
ce que `TreasuryStatement::balance` refuse de faire en le lisant de la dernière
ligne plutôt qu'en le resommant.

### `DomainError::InsufficientTreasury` existe déjà

« Trésorerie insuffisante », `error.rs:21`. Pas de variante neuve : c'est le même
refus, et lui en donner une seconde formulation ferait diverger deux messages
pour une seule règle.

### Aucune garde de phase

Toutes les autres mutations de `teams` en portent une. Celle-ci n'en veut pas —
règle 8 de la phase 1, un ajustement est possible à tout moment. C'est
délibéré et non un oubli : un commissaire corrige souvent **parce que** l'équipe
est bloquée dans une phase, et une garde l'empêcherait de la débloquer.

## L'écriture concurrente : refusée, pas rejouée

`append` reçoit `team.version` ; si quelqu'un a écrit entre la lecture et
l'écriture, le dépôt rend `RepositoryError::ConcurrentWrite`. Le use case le
remonte, le contrôleur en fait un message dans le pied du panneau.

**On ne réessaie pas.** Un ajustement est un acte délibéré, avec un motif écrit
pour un état observé. Le rejouer en silence sur un état qu'on n'a pas relu peut
créditer une caisse que la recette du match qui vient d'être publié a déjà
remplie — et le commissaire validerait alors un geste qu'il n'aurait pas fait
s'il avait vu l'écran. Le cas est rare : deux écritures sur la même équipe à la
même seconde. Le coût du refus est un clic ; celui du rejeu est un solde faux.

Décidé le 2026-10-01.

## Les tests

Le dépôt est mocké — `use_cases/test_doubles.rs` existe déjà. Trois cas :

| Cas | Ce qu'il vérifie |
|---|---|
| équipe introuvable | `TeamNotFound`, et **rien n'est appendé** |
| refus domaine | l'erreur remonte telle quelle, sans être traduite, et rien n'est appendé |
| chemin heureux | l'événement appendé porte le motif, le nom de l'admin et le bon sens |

Le troisième est le seul qui compte vraiment : c'est lui qui attrape un motif
perdu en route, le défaut qui ne se verrait qu'à la lecture du relevé, des
semaines plus tard.

Les règles de validité du montant ne sont **pas** testées ici : elles vivent
dans `AdjustmentAmount`, et c'est là qu'elles s'éprouvent. Les rejouer au niveau
du use case donnerait deux endroits à tenir d'accord.

## Règles métier

**Aucune nouvelle.** Question posée le 2026-10-01 : la phase 5 orchestre des
règles déjà tranchées. Elle en déplace une seule, et vers le bon endroit — le
refus d'un retrait non couvert est au domaine, pas ici.
