# Phase 8 — Cartes kanban — page "Mes équipes"

Remplace intégralement `kanban/ready_to_be_done/44-my-teams-page.md`
(déplacée vers `kanban/cancelled/`, obsolète : filtres/budget retirés,
section archivée ajoutée, statut "Refusée" ajouté, bug du roster non
affiché découvert et corrigé).

## Ordre des cartes (dépendances)

1. **`285-teams-my-teams-repository.md`** — BC `teams`, méthode
   `find_by_coach_and_space` (port + implémentation SQL). Aucune dépendance.
2. **`286-team-card-component-initials.md`** — composant partagé
   `team-card.html` : paramètre `initials`, nouvelles variantes de statut
   CSS. Impacte `competition-teams.html` (contrat partagé). Aucune
   dépendance.
3. **`287-teams-my-teams-widget.md`** — BC `teams`, widget "Mes équipes"
   (route + handler + mapping statut/phase + template + CSS). Dépend de
   285 et 286.
4. **`288-team-creation-my-teams-page.md`** — BC `team_creation`, handler
   et template réécrits (section brouillons uniquement, corrige le bug
   roster jamais affiché). Dépend de 287 (référence sa route via
   `AppRoutes`).
5. **`289-my-teams-e2e-tests.md`** — Tests E2E de la page complète,
   incluant un scénario de non-régression sur le bug initial (statut
   affiché ≠ statut domaine réel). Dépend de 287 et 288.

Chaque carte est compilable et testable indépendamment, dans cet ordre.
