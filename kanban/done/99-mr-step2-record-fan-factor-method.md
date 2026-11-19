# MR-STEP2-03 — Méthode agrégat PreMatch::record_fan_factor

## Objectif

Implémenter la méthode métier `record_fan_factor()` sur l'agrégat PreMatch.

## Dépendances

- MR-STEP2-01, MR-STEP2-02

## Fichier

- `src/app/match_report/domain/match_report_pre_match.rs`

## Conception

Voir `docs/specs/match-report/step2-pre-match/06-domaine.md`

## Checklist

- [ ] Méthode `record_fan_factor(home_fan_roll, away_fan_roll, recorded_by) -> (Self, Event)`
- [ ] Test `record_fan_factor_emet_evenement`
- [ ] Test `record_fan_factor_met_a_jour_les_champs`
