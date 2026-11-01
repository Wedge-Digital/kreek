# Imprimer avec le prochain adversaire

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 1
**Dépend de :** 594, 597, 598
**Spec :** `06-domaine.md` (`next_opponent`, R2, R12), `04-dtos.md` (`NextOpponentOptionVm`), `05-use-cases.md`
**Fichiers :** `src/app/competitions/domain/next_opponent.rs`, `src/app/competitions/use_cases/next_opponent_service.rs`, `src/app/competitions/io/web/widgets/print_next_opponent_widget.rs`, `templates/widgets/print-next-opponent.html`, `src/app/competitions/routes.rs`, `src/app/teams/io/web/widgets/print_preview_widget.rs`, `src/app/teams/routes.rs`

## Objectif

L'option « Mon équipe et mon prochain adversaire » : sa ligne dans la page, et
la seconde fiche dans l'aperçu.

## Conception

- **Domaine** `next_opponent(team, pairings) -> Option<NextOpponent>` : le match
  en cours de saisie d'abord, sinon le premier à venir dans l'ordre des
  journées ; réutilise `Pairing::adversaire_de`. Ne dépend pas de l'ordre SQL.
- **Service** : `find_team_enrollment` → `list_team_matches` → domaine.
  Pas d'inscription ou pas de match : `Unavailable` avec son motif (ce n'est
  pas une erreur).
- **Fragment** `…/competitions/teams/{team_id}/print/next-opponent` : le bouton
  radio, valeur `other_team_id`, et les champs cachés `round_id` et `home`
  (R10) ; ou l'option grisée avec son motif.
- **Aperçu** : route `…/print/preview/with/{other_team_id}` (contrôlée par la
  594), qui ajoute la fiche de l'adversaire (`role = other`).

## Checklist

- [x] `next_opponent` + tests : en cours avant à venir · joués ignorés · position la plus basse d'abord · aucun match · équipe absente des appariements
- [x] Service, fragment, gabarit
- [x] Route de l'aperçu à deux équipes
- [x] `make check-arch`, `make lint`, `make test`
