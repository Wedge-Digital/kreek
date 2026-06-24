# BC match_report — E2E tests step1

**Priorité : haute**
**Dépend de :** 92, 93
**Contexte :** match_report step1, tests Playwright

## Objectif

Écrire les tests E2E Playwright pour la page step1 du rapport de match.

## Conception

Cf. `docs/specs/match-report/step1-selection/07-integration.md`

### Fichier

`tests/e2e/test_match_report_selection.py`

### Scénarios

| Test | Description |
|------|-------------|
| `test_create_match_report_from_scratch` | Coach arrive sur `/match-report/new`, sélectionne compétition/saison/journée/équipes, clique Commencer → redirect vers step2 |
| `test_prefilled_match_report_from_pairing` | Un pairing existe → le coach arrive sur `/match-report/{id}` → formulaire pré-rempli, bannière verte visible, clique Commencer → redirect step2 |
| `test_same_team_error` | Sélectionne la même équipe home et away → message d'erreur affiché |
| `test_cascade_selects` | Change la compétition → les saisons se rechargent, change la saison → les journées se rechargent |
| `test_resume_returns_to_correct_step` | Un rapport en phase PreMatch → arriver sur `/match-report/{id}` redirige vers step2, pas step1 |
| `test_coach_only_sees_own_teams` | Coach lambda → le select "mon équipe" ne contient que ses propres équipes enrolled en ReadyToPlay |

### Prérequis données

Les tests nécessitent des données de seed :
- Au moins une compétition avec une saison active
- Des journées avec des pairings
- Des équipes enrolled en ReadyToPlay
- Un compte coach + un compte admin

Utiliser les fixtures existantes ou créer des fixtures spécifiques via le seed CLI.

## Checklist

- [ ] `test_create_match_report_from_scratch`
- [ ] `test_prefilled_match_report_from_pairing`
- [ ] `test_same_team_error`
- [ ] `test_cascade_selects`
- [ ] `test_resume_returns_to_correct_step`
- [ ] `test_coach_only_sees_own_teams`
- [ ] `make e2e` passe (avec serveur dev lancé)
