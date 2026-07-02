# MR-STEP5-03 — Use case `record_post_match`

## Objectif

Implémenter le use case qui orchestre la soumission du step 5 : charger l'agrégat,
appeler la méthode domaine, persister l'événement.

## Dépendances

140 — `MatchReportPreMatch::record_post_match()` et `MatchReportReadyToPublish::record_post_match()`
doivent exister.

## Conception

Voir `docs/specs/match-report/step5-apres-match/05-use-cases.md`.

## Fichiers impactés

- `src/app/match_report/use_cases/record_post_match_use_case.rs` (nouveau)
- `src/app/match_report/use_cases/mod.rs`

## Checklist

- [ ] `RecordPostMatchCommand` — struct avec `match_report_id`, `home_gain`, `away_gain`, `home_fan_mod`, `away_fan_mod`, `summary_title`, `summary_body`, `recorded_by`
- [ ] `RecordPostMatchOutcome` — enum `{ Success }`
- [ ] `RecordPostMatchError` — enum `{ NotFound, NotInCompatibleState, Internal(String) }`
- [ ] `execute()` — orchestration : load → vérifier état (`PreMatch` ou `ReadyToPublish`) → appeler méthode domaine → `append_event`
- [ ] Déclaration du module dans `use_cases/mod.rs`
- [ ] Compiler sans erreur (`cargo build`)
