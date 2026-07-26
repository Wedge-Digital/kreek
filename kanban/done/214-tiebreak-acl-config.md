# Départages — Propagation ACL de la configuration vers `ranking`

**Priorité : haute**
**Dépend de :** — (la configuration existe déjà côté `competitions` depuis la carte 211)
**Contexte :** `src/app/ranking/ports.rs`, `src/infrastructure/ranking/competition_info_adapter.rs`
**Spec :** `docs/specs/ranking/tiebreakers/tiebreak-calc/{03-back,04-dtos}.md`

## Objectif

Faire remonter la configuration de départage de `competitions` jusqu'au DTO de port du BC
`ranking`. **Purement additif** : la donnée est transportée mais lue par personne — le
calcul arrive en carte 218. Commit intermédiaire sûr.

Même schéma que la carte 204 (propagation ACL des bonus) de la feature
`ranking-bonus-points`.

## Conception (cf. `04-dtos.md`)

### `ranking/ports.rs`

```rust
pub struct TiebreakSettingInfo {
    pub code: String,
    pub activated: bool,
}
// RankingRulesInfo : + pub tiebreakers: Vec<TiebreakSettingInfo>
```

L'ordre du `Vec` **est** la priorité, comme côté `competitions` — aucun champ de rang.
Primitives assumées : DTO de lecture.

### `infrastructure/ranking/competition_info_adapter.rs`

`find_ranking_rules` recopie `competitions::RankingRules.tiebreakers` :
`TiebreakConfig::settings()` → `Vec<TiebreakSettingInfo>`, en préservant l'ordre.
`code` via `TiebreakCode::as_ref()`, `activated` via `Activated.0`.

### Littéraux `RankingRulesInfo` dans les tests (fix compilation)

Ajouter le champ aux constructions existantes, avec `vec![]` (aucun départage) pour ne
rien changer au comportement :

- `record_match_ranking_use_case.rs` (module tests, `FakeCompetitionPort`)
- `tests/test_match_report_published_pipeline.rs` (`FakeCompetitionPort`)
- `classement_widget.rs` / `builders.rs` si un littéral y figure

## Checklist

- [ ] `TiebreakSettingInfo` défini ; `RankingRulesInfo` enrichi
- [ ] Adapter recopie la configuration en préservant l'ordre
- [ ] Littéraux de test complétés avec `vec![]`
- [ ] Test de l'adapter : une configuration à 3 critères dont 1 décoché est recopiée à
      l'identique, ordre compris
- [ ] `make test` + `make check-arch` passent
