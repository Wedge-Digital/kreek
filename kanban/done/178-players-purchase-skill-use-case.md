# BC `players` — Use case d'achat de compétence

**Priorité : haute**
**Dépend de :** `175-players-domain-improvement.md`, `177-players-skill-catalog-port.md`
**Contexte :** `players/use_cases` + `players/io/web`

## Objectif

Câbler l'achat d'une compétence : résolution du coût réel (jamais celui
soumis par le client), vérification d'accès à la catégorie, appel à la
méthode domaine, persistance. Spec complète :
`docs/specs/player-spp-spending/README.md`.

---

## Conception

### Domain service (`use_cases/improvement_cost_service.rs`)

```rust
pub enum ImprovementCostError { SkillNotFound, PositionNotFound, CategoryNotAccessible }

pub fn resolve_skill_cost(
    catalog: &dyn ISkillCatalogPort, roster_line_id: &str, skill_id: &str,
    mode: AcquisitionMode, level: u8,
) -> Result<(SppCost, ValueKpo), ImprovementCostError> {
    let skill = catalog.find_skill(skill_id).ok_or(ImprovementCostError::SkillNotFound)?;
    let access = catalog.position_access(roster_line_id).ok_or(ImprovementCostError::PositionNotFound)?;
    let is_secondary = access.secondary_categories.contains(&skill.category);
    let is_primary   = access.primary_categories.contains(&skill.category);
    if !is_primary && !is_secondary { return Err(ImprovementCostError::CategoryNotAccessible); }
    let level_cost = catalog.cost_for_level(level).expect("niveau plafonné à 6, toujours défini");
    let cost = match mode {
        AcquisitionMode::Chosen if skill.is_elite => level_cost.chosen_elite.unwrap_or(if is_secondary { level_cost.chosen_secondary } else { level_cost.chosen_primary }),
        AcquisitionMode::Chosen => if is_secondary { level_cost.chosen_secondary } else { level_cost.chosen_primary },
        AcquisitionMode::Random if skill.is_elite => level_cost.random_elite.unwrap_or(level_cost.random),
        AcquisitionMode::Random => level_cost.random,
    };
    let value_delta = catalog.skill_value_delta(is_secondary);
    Ok((SppCost::try_new(cost as u8).expect("coût borné par la matrice"), ValueKpo(value_delta)))
}

pub fn resolve_stat_cost(catalog: &dyn ISkillCatalogPort, stat: StatKind, level: u8) -> (SppCost, ValueKpo) {
    let cost = catalog.cost_for_level(level).expect("niveau plafonné à 6").characteristic;
    (SppCost::try_new(cost as u8).expect("coût borné"), ValueKpo(catalog.stat_value_delta(stat)))
}
```

### Commande + use case

```rust
pub struct PurchaseSkillCommand { pub player_id: PlayerId, pub skill_id: SkillId, pub mode: AcquisitionMode }

pub enum PurchaseSkillError { PlayerNotFound, Cost(ImprovementCostError), Domain(DomainError), Repository(RepositoryError) }

pub async fn execute(cmd: PurchaseSkillCommand, player_repo: &dyn IPlayerRepository, catalog: &dyn ISkillCatalogPort) -> Result<(), PurchaseSkillError> {
    let player = player_repo.find_by_id(&cmd.player_id).await.map_err(PurchaseSkillError::Repository)?.ok_or(PurchaseSkillError::PlayerNotFound)?;
    let skill = catalog.find_skill(&cmd.skill_id.to_string()).ok_or(PurchaseSkillError::Cost(ImprovementCostError::SkillNotFound))?;
    let level = player.next_improvement_level();
    let (cost, value_delta) = resolve_skill_cost(catalog, &player.roster_line_id.to_string(), &cmd.skill_id.to_string(), cmd.mode, level)
        .map_err(PurchaseSkillError::Cost)?;
    let skill_name = SkillName::try_new(skill.name).expect("nom déjà validé côté référence");
    let event = player.purchase_skill(cmd.skill_id, skill_name, cmd.mode, cost, value_delta).map_err(PurchaseSkillError::Domain)?;
    player_repo.append(&player.id, &player.team_id, &event, player.version).await.map_err(PurchaseSkillError::Repository)?;
    Ok(())
}
```

### Handler (`io/web/purchase_skill_controller.rs`)

Vérifie phase + permission (accès direct à `teams::Team`, même précédent
que `check_admin_rights` dans `player_detail_controller.rs`) **avant**
d'appeler le use case — pas dans le use case lui-même. Réponse
`HX-Refresh: true` en succès.

### Route

```
POST /app/{space_id}/players/{player_id}/skills   (body: skill_id, mode)
```

---

## Checklist

- [ ] `improvement_cost_service::resolve_skill_cost` + `resolve_stat_cost` (partagé avec carte 179)
- [ ] `PurchaseSkillCommand` + `purchase_skill_use_case::execute`
- [ ] Handler avec vérification phase + permission (coach/admin) avant appel use case
- [ ] Route POST câblée
- [ ] Tests : coût recalculé ignore un coût client falsifié, catégorie hors accès → erreur, niveau croissant après achats successifs
