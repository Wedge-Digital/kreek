# MR-STEP2-02 — Événement FanFactorRecorded + rehydratation

## Objectif

Ajouter l'événement domaine `FanFactorRecorded` et sa prise en charge dans la machine d'états + projection.

## Dépendances

- MR-STEP2-01 (D3Roll)

## Fichiers

- `src/app/match_report/domain/events.rs` — variante `FanFactorRecorded`
- `src/app/match_report/domain/match_report_pre_match.rs` — champs `home_fan_roll`, `away_fan_roll`
- `src/app/match_report/domain/match_report_state.rs` — bras rehydratation
- `src/app/match_report/io/repository/match_report_repository.rs` — bras vide projection

## Conception

Voir `docs/specs/match-report/step2-pre-match/06-domaine.md`

## Checklist

- [ ] Variante `FanFactorRecorded { home_fan_roll: D3Roll, away_fan_roll: D3Roll, recorded_by: CoachId }`
- [ ] `type_name()` → `"FanFactorRecorded"`
- [ ] Champs `Option<D3Roll>` sur `MatchReportPreMatch`
- [ ] `from_draft()` initialise à `None`
- [ ] Bras rehydratation (dernier événement écrase)
- [ ] Bras vide dans `update_projection_in_tx`
- [ ] Test `rehydratation_fan_factor`
- [ ] Test `rehydratation_double_fan_factor`
