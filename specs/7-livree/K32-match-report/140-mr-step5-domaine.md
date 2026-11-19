# MR-STEP5-02 — Domaine step5

## Objectif

Implémenter toute la logique domaine du step 5 : value objects, méthodes d'agrégat,
nouvel agrégat `MatchReportReadyToPublish`, événement `PostMatchRecorded`, réhydration.

## Dépendances

139 — les champs `home_dedicated_fans` / `away_dedicated_fans` doivent être présents
sur `MatchReportPreMatch` avant d'implémenter `suggest_gains()`.

## Conception

Voir `docs/specs/match-report/step5-apres-match/06-domaine.md`.

## Fichiers impactés

- `src/app/match_report/domain/value_objects.rs`
- `src/app/match_report/domain/events.rs`
- `src/app/match_report/domain/match_report_pre_match.rs`
- `src/app/match_report/domain/match_report_ready_to_publish.rs` (nouveau)
- `src/app/match_report/domain/match_report_state.rs`
- `src/app/match_report/domain/mod.rs`

## Checklist

### Value objects
- [ ] `MatchGain(u32)` — `nutype`, `validate(greater = 0)`, derive `Serialize, Deserialize`
- [ ] `FanFactorMod(i8)` — `nutype`, `validate(greater_or_equal = -2, less_or_equal = 2)`, derive `Serialize, Deserialize`

### Événement
- [ ] `PostMatchRecorded { home_gain, away_gain, home_fan_mod, away_fan_mod, summary_title, summary_body, recorded_by }` dans `events.rs`
- [ ] `type_name()` et `schema_version()` mis à jour

### Méthodes sur `MatchReportPreMatch`
- [ ] `compute_score() -> (u8, u8)` — compte `Touchdown` par side
- [ ] `compute_cas() -> (u8, u8)` — compte `Sortie` par side uniquement
- [ ] `suggest_gains() -> (u32, u32)` — formule `(fans_home + fans_away) / 2 × 10 000 + tds × 10 000`
- [ ] `record_post_match(...) -> (MatchReportReadyToPublish, MatchReportDomainEvent)`

### Nouvel agrégat `MatchReportReadyToPublish`
- [ ] Struct avec tous les champs de `MatchReportPreMatch` + champs post-match (`home_gain`, `away_gain`, `home_fan_mod`, `away_fan_mod`, `summary_title`, `summary_body`)
- [ ] `from_pre_match(pm, ...)` — constructeur depuis `MatchReportPreMatch`
- [ ] `record_post_match(...) -> (Self, MatchReportDomainEvent)` — re-soumission
- [ ] Déclaration dans `domain/mod.rs`

### État
- [ ] Variante `ReadyToPublish(MatchReportReadyToPublish)` dans `MatchReportState`
- [ ] Réhydration de `PostMatchRecorded` sur `PreMatch` → `ReadyToPublish`
- [ ] Réhydration de `PostMatchRecorded` sur `ReadyToPublish` → `ReadyToPublish` (mise à jour)

### Tests unitaires (dans `match_report_pre_match.rs`)
- [ ] `compute_score_counts_touchdowns_by_side`
- [ ] `compute_score_ignores_other_actions`
- [ ] `compute_cas_counts_only_sortie`
- [ ] `compute_cas_ignores_blesse`
- [ ] `suggest_gains_applies_formula`
- [ ] `suggest_gains_with_zero_tds`
- [ ] `suggest_gains_integer_division`
- [ ] `record_post_match_emits_correct_event`
- [ ] `fan_factor_mod_rejects_out_of_range`
- [ ] `fan_factor_mod_accepts_boundaries`
- [ ] `match_gain_rejects_zero`
- [ ] `match_gain_accepts_positive`

### Build
- [ ] Compiler sans erreur (`cargo build`)
- [ ] Tests verts (`cargo test`)
