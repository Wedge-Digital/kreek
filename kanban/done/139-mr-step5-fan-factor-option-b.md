# MR-STEP5-01 — Enrichissement fan factor (Option B)

## Objectif

Stocker les `dedicated_fans` dans l'événement `FanFactorRecorded` afin que l'agrégat
puisse calculer `suggest_gains()` de manière autonome (sans appel de port).

## Dépendances

Aucune.

## Conception

Voir `docs/specs/match-report/step5-apres-match/06-domaine.md` — section « Impact sur le flow step 2 ».

## Fichiers impactés

- `src/app/match_report/domain/events.rs`
- `src/app/match_report/domain/match_report_pre_match.rs`
- `src/app/match_report/domain/match_report_state.rs`
- `src/app/match_report/ports.rs`
- `src/app/match_report/use_cases/record_fan_factor_use_case.rs`
- `src/app/match_report/io/web/pre_match_controller.rs`
- `src/infrastructure/match_report/` (adapteur `ITeamDataPort`)

## Checklist

- [ ] `TeamInfoDto` — ajouter le champ `dedicated_fans: u32`
- [ ] Adapteur `ITeamDataPort` — retourner `dedicated_fans` depuis la source BC Teams
- [ ] `FanFactorRecorded` — ajouter `#[serde(default)] home_dedicated_fans: u32` et `away_dedicated_fans: u32`
- [ ] `MatchReportPreMatch` — ajouter les champs `home_dedicated_fans: u32` et `away_dedicated_fans: u32`
- [ ] Réhydration `FanFactorRecorded` dans `match_report_state.rs` — alimenter les nouveaux champs
- [ ] `MatchReportPreMatch::record_fan_factor()` — accepter `home_dedicated_fans` et `away_dedicated_fans` en paramètres
- [ ] `RecordFanFactorCommand` — ajouter `home_dedicated_fans: u32` et `away_dedicated_fans: u32`
- [ ] `record_fan_factor_use_case::execute` — fetcher `dedicated_fans` via `find_team_info`, les passer à la commande
- [ ] `post_pre_match` handler — passer les `dedicated_fans` à la commande
- [ ] `MatchReportPreMatch::from_draft()` — initialiser `home_dedicated_fans` et `away_dedicated_fans` à `0`
- [ ] Compiler sans erreur (`cargo build`)
- [ ] Tests existants toujours verts (`cargo test`)
