# BC match_report — E2E tests step2-inducements

**Priorité : haute**
**Dépend de :** 112
**Contexte :** match_report step2-inducements — tests Playwright

## Objectif

Écrire les tests E2E Playwright pour la page d'achat des inducements.

## Conception

Cf. `docs/specs/match-report/step2-inducements/02-front.md`

### Fichier

`tests/e2e/test_match_report_inducements.py`

### Scénarios

| Test | Description |
|---|---|
| `test_topdog_buys_inducements_and_redirects_to_underdog` | TopDog ajoute des inducements, clique Valider → redirect vers page Underdog |
| `test_topdog_passes_redirects_to_underdog` | TopDog clique Passer → redirect vers page Underdog, budget Underdog inclut 0 dépenses TopDog |
| `test_underdog_budget_reflects_topdog_spending` | TopDog achète pour 50k → budget affiché pour Underdog inclut ces 50k |
| `test_underdog_buys_and_redirects_to_step3` | Underdog achète, Valider → redirect vers step3 |
| `test_budget_exceeded_disables_submit` | Sélection dépasse le budget → bouton Valider désactivé |
| `test_no_inducements_skips_phase` | Compétition sans inducements → step2 POST redirige directement vers step3 (phase sautée) |
| `test_star_player_conflict_rejected` | TopDog choisit un star player déjà choisi par l'adversaire → rejeté (back) |

### Prérequis données

- Un rapport en état `PreMatch` avec `FanFactorRecorded` + `TeamValuesRecorded`
- Une compétition avec tier rules contenant des inducements
- Une compétition sans inducements pour tester le skip
- Deux équipes avec des trésoreries différentes

## Checklist

- [ ] `test_topdog_buys_inducements_and_redirects_to_underdog`
- [ ] `test_topdog_passes_redirects_to_underdog`
- [ ] `test_underdog_budget_reflects_topdog_spending`
- [ ] `test_underdog_buys_and_redirects_to_step3`
- [ ] `test_budget_exceeded_disables_submit`
- [ ] `test_no_inducements_skips_phase`
- [ ] `test_star_player_conflict_rejected`
- [ ] `make e2e` passe (avec serveur dev lancé)
