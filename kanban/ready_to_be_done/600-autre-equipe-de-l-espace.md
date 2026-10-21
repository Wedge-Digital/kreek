# Imprimer avec une autre équipe de l'espace

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 1
**Dépend de :** 599 (route d'aperçu à deux équipes)
**Spec :** `02-front.md` (« Choix d'une autre équipe », R3), `04-dtos.md` (`TeamPrintPageVm`)
**Fichiers :** `src/app/teams/io/web/team_print_page.rs`, `templates/teams-print-page.html`

## Objectif

L'option « Mon équipe et une autre équipe » : compétition → saison → équipe,
dans l'espace de mon équipe, parmi les équipes prêtes à jouer.

## Conception

- Trois `kreek-select` en cascade sur les routes JSON **existantes** :
  `json/competitions`, `json/seasons` (`auto-select-first`),
  `team-selection/json` (qui ne rend que les équipes `ReadyToPlay`).
- Événements préfixés : `printCompetitionSelected`, `printSeasonSelected`.
- Compétition et saison de mon équipe présélectionnées.
- L'équipe choisie part dans le **chemin** de l'aperçu
  (`/with/{other_team_id}`) : la 594 vérifie qu'elle appartient à l'espace,
  ce que la route `team-selection/json` ne garantit pas (hors périmètre).
- URLs fabriquées par `AppRoutes` (`TeamPrintPageVm`), aucune en dur (axe 4).

## Checklist

- [ ] Cascade dans la page, présélection
- [ ] Le choix d'une équipe recharge l'aperçu avec sa fiche
- [ ] Test unitaire du VM : URLs des trois listes et de l'aperçu
- [ ] `make check-arch`, `make lint`, `make test`
