# MR-STEP2-04 — Use case record_fan_factor

## Objectif

Orchestration : charge agrégat, appelle méthode domaine, persiste événement.

## Dépendances

- MR-STEP2-03

## Fichiers

- `src/app/match_report/use_cases/record_fan_factor_use_case.rs` (nouveau)
- `src/app/match_report/use_cases/mod.rs`

## Conception

Voir `docs/specs/match-report/step2-pre-match/05-use-cases.md`

## Checklist

- [ ] Struct `RecordFanFactorCommand`
- [ ] Enum `RecordFanFactorError` (NotFound, NotInPreMatchPhase, Repository)
- [ ] Fonction `execute()` : find_by_id → vérifier PreMatch → record_fan_factor → append
- [ ] Retourne `MatchReportId`
