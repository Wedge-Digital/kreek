# MR-STEP2-06 — Page step 2 : handler GET + template

## Objectif

Afficher la page d'avant-match avec le banner match, les sections informatives (fan factor, journaliers, TV, inducements), et le formulaire de saisie des D3.

## Dépendances

- MR-STEP2-05 (endpoint JSON Teams)

## Fichiers

- `src/app/match_report/io/web/pre_match_controller.rs` (nouveau)
- `src/app/match_report/io/web/templates/pre-match.html` (nouveau)
- `src/app/match_report/io/web/mod.rs`
- `src/app/match_report/routes.rs`
- `src/app/match_report/router.rs`

## Conception

Voir `docs/specs/match-report/step2-pre-match/07-integration.md`

## Checklist

- [ ] Struct `PreMatchTemplate` avec URLs JSON bakées
- [ ] Handler `get_pre_match` : charge PreMatch, redirect si Draft/Cancelled
- [ ] Route `MATCH_REPORT_STEP2` + builder `step2()`
- [ ] Template : header, steps, match banner (rendu serveur)
- [ ] Template : sections Alpine avec fetch() des données d'équipe
- [ ] Template : calcul temps réel fan factor, diff TV, ordre inducements
- [ ] Template : formulaire avec inputs D3 (min=1, max=3)
- [ ] CSS : reprendre les styles de la maquette `app-match-report-step2.html`
