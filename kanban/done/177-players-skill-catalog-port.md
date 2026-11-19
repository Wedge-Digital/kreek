# BC `players` — Port ACL `ISkillCatalogPort` vers `references`

**Priorité : haute**
**Dépend de :** `176-references-improvement-value-data.md`
**Contexte :** `players/ports.rs` + `src/infrastructure/players/`

## Objectif

Donner à `players` un accès propre (port + adaptateur) à la matrice de coût
et à la table de valeur d'amélioration de `references`, sans jamais
dépendre directement de `references::domain::port::IReferenceRepository`
dans le code applicatif de `players` (règle CLAUDE.md « Adapters
inter-BCs »). Spec complète : `docs/specs/player-spp-spending/README.md`.

---

## Conception

### Port (`players/ports.rs`)

```rust
pub struct SkillCatalogEntryDto { pub skill_id: String, pub name: String, pub category: String, pub is_elite: bool }
pub struct PositionAccessDto    { pub primary_categories: Vec<String>, pub secondary_categories: Vec<String> }
pub struct SkillCostLevelDto {
    pub level: u8,
    pub chosen_primary: u32, pub chosen_secondary: u32,
    pub chosen_elite: Option<u32>, pub random: u32, pub random_elite: Option<u32>,
    pub characteristic: u32,
}

pub trait ISkillCatalogPort: Send + Sync {
    fn find_skill(&self, skill_id: &str) -> Option<SkillCatalogEntryDto>;
    fn position_access(&self, roster_line_id: &str) -> Option<PositionAccessDto>;
    fn cost_for_level(&self, level: u8) -> Option<SkillCostLevelDto>;
    fn skill_value_delta(&self, is_secondary_access: bool) -> u32;
    fn stat_value_delta(&self, stat: crate::app::players::domain::match_impact::StatKind) -> u32;
}
```

Port synchrone (comme `IReferenceRepository` lui-même — pas d'I/O, données
chargées en mémoire au démarrage).

### Adaptateur (`src/infrastructure/players/skill_catalog_adapter.rs`)

```rust
pub struct SkillCatalogAdapter { reference_repo: Arc<dyn IReferenceRepository> }
impl ISkillCatalogPort for SkillCatalogAdapter {
    fn find_skill(&self, skill_id: &str) -> Option<SkillCatalogEntryDto> {
        let s = self.reference_repo.find_skill_by_uid(skill_id)?;
        Some(SkillCatalogEntryDto { skill_id: s.uid.clone(), name: s.name.clone(), category: s.category.clone(), is_elite: s.skill_type == "Élite" })
    }
    // position_access() résout via find_position_by_uid() → primary_access/secondary_access
    // cost_for_level() mappe SkillCostLevel → SkillCostLevelDto
    // skill_value_delta()/stat_value_delta() lisent ImprovementValueTable
}
```

### Câblage

`PlayersContext` (`players/context.rs`) gagne `skill_catalog: Arc<dyn ISkillCatalogPort>`,
instancié dans `main.rs` comme les autres adaptateurs inter-BC existants.

---

## Checklist

- [ ] `ISkillCatalogPort` + 3 DTOs dans `players/ports.rs`
- [ ] `SkillCatalogAdapter` dans `src/infrastructure/players/skill_catalog_adapter.rs`
- [ ] Câblage dans `PlayersContext` + `main.rs`
- [ ] Aucun import de `references::domain::*` en dehors de l'adaptateur (vérifié par `check-arch` à terme)
- [ ] Tests sur l'adaptateur : mapping correct is_elite, position_access absente → None, cost_for_level hors bornes → None
