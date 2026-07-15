# BC `players` — Domaine : achat de compétences et augmentations de caractéristiques

**Priorité : haute**
**Dépend de :** rien (domaine pur)
**Contexte :** `players/domain` — agrégat `Player`

## Objectif

Étendre l'agrégat `Player` pour qu'il puisse encaisser un achat de
compétence additionnelle ou une augmentation de caractéristique payée en
SPP. Spec complète : `docs/specs/player-spp-spending/README.md`.

---

## Conception

### Nouveau champ

```rust
pub stat_increases: Vec<StatIncrease>,   // distinct de stat_adjustments (malus de blessure)

pub struct StatIncrease {
    pub stat: StatKind,       // réutilise match_impact::StatKind (Ma|St|Ag|Pa|Av)
    pub spp_cost: SppCost,
    pub value_delta: ValueKpo,
}
```
`AcquiredSkill` (existant) gagne `pub value_delta: ValueKpo`.

### Nouveaux événements

```rust
PlayerSkillPurchased { skill_id: SkillId, skill_name: SkillName, mode: AcquisitionMode, spp_cost: SppCost, value_delta: ValueKpo }
PlayerStatIncreased  { stat: StatKind, spp_cost: SppCost, value_delta: ValueKpo }
```

### Méthodes domaine (infaillibles → deviennent faillibles, rupture de convention assumée)

```rust
pub fn spp_remaining(&self) -> u32 {
    let spent: u32 = self.acquired_skills.iter().map(|s| s.spp_cost.into_inner() as u32).sum::<u32>()
        + self.stat_increases.iter().map(|s| s.spp_cost.into_inner() as u32).sum::<u32>();
    self.spp.0.saturating_sub(spent)
}

pub fn next_improvement_level(&self) -> u8 {
    ((self.acquired_skills.len() + self.stat_increases.len()) as u8 + 1).min(6)
}

pub fn purchase_skill(&self, skill_id: SkillId, skill_name: SkillName, mode: AcquisitionMode, spp_cost: SppCost, value_delta: ValueKpo) -> Result<PlayerDomainEvent, DomainError> {
    if self.base_skills.contains(&skill_id) || self.acquired_skills.iter().any(|s| s.skill_id == skill_id) {
        return Err(DomainError::SkillAlreadyAcquired);
    }
    if self.spp_remaining() < spp_cost.into_inner() as u32 {
        return Err(DomainError::InsufficientSpp);
    }
    Ok(PlayerDomainEvent::PlayerSkillPurchased { skill_id, skill_name, mode, spp_cost, value_delta })
}

pub fn increase_stat(&self, stat: StatKind, spp_cost: SppCost, value_delta: ValueKpo) -> Result<PlayerDomainEvent, DomainError> {
    if self.spp_remaining() < spp_cost.into_inner() as u32 {
        return Err(DomainError::InsufficientSpp);
    }
    Ok(PlayerDomainEvent::PlayerStatIncreased { stat, spp_cost, value_delta })
}
```

`spp_remaining()` remplace le calcul aujourd'hui dupliqué dans
`player_detail_controller.rs::compute_spp_breakdown` — c'est une règle
domaine, pas une préoccupation web. Le contrôleur sera mis à jour dans la
carte 181 pour appeler cette méthode au lieu de recalculer.

### `apply()` — nouvelles branches

```rust
PlayerDomainEvent::PlayerSkillPurchased { skill_id, skill_name, mode, spp_cost, value_delta } => {
    self.acquired_skills.push(AcquiredSkill { skill_id: skill_id.clone(), skill_name: skill_name.clone(), mode: *mode, spp_cost: *spp_cost, value_delta: *value_delta });
    self.value.0 += value_delta.0;
}
PlayerDomainEvent::PlayerStatIncreased { stat, spp_cost, value_delta } => {
    self.stat_increases.push(StatIncrease { stat: *stat, spp_cost: *spp_cost, value_delta: *value_delta });
    self.value.0 += value_delta.0;
}
```

### `DomainError` (aujourd'hui vide — premières règles)

```rust
pub enum DomainError {
    SkillAlreadyAcquired,
    InsufficientSpp,
}
```

---

## Checklist

- [ ] `StatIncrease` + champ `stat_increases` sur `Player`
- [ ] `AcquiredSkill.value_delta` ajouté
- [ ] 2 nouveaux événements + branches `apply()`
- [ ] `spp_remaining()` + `next_improvement_level()` sur `Player`
- [ ] `purchase_skill()` / `increase_stat()` avec gardes (déjà acquis, SPP insuffisant)
- [ ] `DomainError::SkillAlreadyAcquired` / `InsufficientSpp`
- [ ] Tests unitaires : achat compétence nominal, achat déjà possédée (base et acquise) → erreur, SPP insuffisant → erreur, `next_improvement_level` plafonne à 6, `spp_remaining` correcte après plusieurs achats mixtes (compétences + stats), `value` incrémentée à chaque achat
