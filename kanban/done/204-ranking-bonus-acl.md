# Ranking — Propagation ACL des 3 bonus (ports + adapter)

**Priorité : haute**
**Dépend de :** — (unité 1 livrée : `competitions::RankingRules` porte déjà les 3 bonus)
**Contexte :** `src/app/ranking/ports.rs`, `src/infrastructure/ranking/competition_info_adapter.rs`
**Spec :** `docs/specs/ranking-bonus-points/post-match-bonus-calc/{03-back,04-dtos,07-integration}.md`

## Objectif

Faire remonter la config des 3 bonus (offensif/défensif/agressif) du BC
`competitions` jusqu'au DTO de port `RankingRulesInfo` du BC `ranking`. **Aucun
comportement modifié** à ce stade : la donnée est transportée mais pas encore lue par
le domaine (le calcul arrive en carte 206). Commit intermédiaire sûr.

## Conception (cf. `04-dtos.md`, `07-integration.md` §2)

### `ports.rs` — sous-DTO générique + enrichissement
```rust
pub struct BonusRuleInfo {
    pub activated: bool,
    pub threshold: u32, // min_td (off) | max_td_conceded (def) | min_casualties (agg)
    pub points: u32,
}
// RankingRulesInfo : + offensive / defensive / aggressive: BonusRuleInfo
```
Primitifs acceptés (DTO de lecture, règle CQRS).

### Adapter — recopie depuis `competitions`
`find_ranking_rules` mappe chaque bonus de `competitions::domain::RankingRules` (VOs
nutype, `.into_inner()`) vers `BonusRuleInfo` :
- `offensive_bonus.min_td` → `offensive.threshold`
- `defensive_bonus.max_td_conceded` → `defensive.threshold`
- `aggressive_bonus.min_casualties` → `aggressive.threshold`
- `activated` / `points` → `activated` / `points`

### Littéraux `RankingRulesInfo` dans les tests (fix compilation)
Ajouter les 3 champs bonus aux constructions existantes :
- `record_match_ranking_use_case.rs` (module tests, `FakeCompetitionPort`)
- `tests/test_match_report_published_pipeline.rs` (`FakeCompetitionPort`)
Valeurs par défaut = bonus désactivés (`activated: false`) → aucun effet.

## Checklist

- [ ] `BonusRuleInfo` défini ; `RankingRulesInfo` enrichi des 3 bonus
- [ ] Adapter `find_ranking_rules` recopie les 3 bonus (mapping des seuils correct)
- [ ] Littéraux `RankingRulesInfo` des tests complétés (bonus désactivés)
- [ ] `classement_widget::build_vm` compile sans changement (n'utilise que `is_none()`)
- [ ] `make test` + `make check-arch` passent
