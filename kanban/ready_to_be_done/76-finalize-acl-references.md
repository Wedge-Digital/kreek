# BC `team_creation` — ACL finalize-team : suppression des violations `references`

**Priorité : haute (bloquant pour 77–80)**
**Dépend de :** `66-tc-acl-reference-data-port.md` (pattern ACL déjà en place)
**Contexte :** BC `team_creation` — handler finalize_team

## Objectif

Migrer `finalize_team.rs` pour utiliser `IReferenceDataPort` au lieu d'accéder directement à `state.references.repository`. Supprimer toutes les violations architecturales cross-BC. Aucun changement fonctionnel.

---

## Situation actuelle

`finalize_team.rs` importe directement :
- `crate::app::references::domain::models::ChosenSkillCost`
- `crate::app::references::routes::Routes as RefRoutes`
- `state.references.repository.as_ref()` — 2 accès (GET handler + POST handler)

Données consommées depuis le BC `references` :
- `list_teams()` → roster leagues (pour auto-skip)
- `skill_cost_matrix()` → pricing (pour le JSON et le POST)
- `find_position_by_uid()` → base skills des joueurs + accès primaire/secondaire (GET + POST)
- `find_skill_by_uid()` → noms des skills de base (GET) + catégorie (POST)
- `skill_picker_base()` → URL du widget (GET)

---

## Plan

### Méthodes à ajouter au port `IReferenceDataPort`

Le port a déjà `find_roster_definition()` (qui contient les leagues) et `resolve_skill_cost()` (carte 58). Il manque :

1. `resolve_base_skills(roster_line_id: &str) -> Vec<String>` — retourne les noms des skills de base d'une position
2. `skill_pricing_level_1() -> Option<SkillPricingDefinition>` — retourne les coûts SPP niveau 1

`SkillPricingDefinition` est un DTO du port :
```rust
pub struct SkillPricingDefinition {
    pub chosen_primary: u8,
    pub chosen_secondary: u8,
    pub random: u8,
}
```

### Migrations dans le handler GET

- Remplacer `ref_repo.list_teams()...find(roster_uid).leagues` par `ref_data.find_roster_definition(roster_uid).leagues`
- Remplacer `ref_repo.skill_cost_matrix()` par `ref_data.skill_pricing_level_1()`
- Remplacer `ref_repo.find_position_by_uid()...skills` par `ref_data.resolve_base_skills()`
- Remplacer `RefRoutes::default().skill_picker_base()` par la constante path du BC `references` passée en paramètre au template (ou baked dans le template via `ref_routes`)

### Migration dans le handler POST

- Remplacer `ref_repo.find_skill_by_uid()` + `ref_repo.find_position_by_uid()` + calcul de coût par `ref_data.resolve_skill_cost()` (déjà implémenté carte 58)

---

## Situation finale

- **Aucun import de `crate::app::references`** dans `finalize_team.rs`
- `check-arch` ne signale plus de violation pour `finalize_team.rs`
- Fonctionnellement identique à avant

---

## Checklist

- [ ] Ajouter `resolve_base_skills()` au trait `IReferenceDataPort`
- [ ] Ajouter `skill_pricing_level_1()` au trait `IReferenceDataPort` + DTO `SkillPricingDefinition`
- [ ] Implémenter dans `reference_data_adapter.rs`
- [ ] Mettre à jour les fakes (post_login, roster_service tests)
- [ ] Migrer le handler GET : remplacer tous les accès `state.references.repository`
- [ ] Migrer le handler POST : utiliser `resolve_skill_cost()` au lieu du calcul manuel
- [ ] Supprimer les imports `references::*` de `finalize_team.rs`
- [ ] `check-arch` — aucune violation pour `finalize_team.rs`
- [ ] `cargo check` — 0 erreur
