# La feuille de match imprimable

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 2
**Dépend de :** 602, 605, 596, 594
**Spec :** `04-dtos.md` (`MatchSheetVm`), `05-use-cases.md` (`match_sheet_service`), R4, R6, R9, R10, R11
**Maquette :** feuille de match de `assets/rawpages/html/app-team-print-modal.html`
**Fichiers :** `src/app/match_report/use_cases/match_sheet_service.rs`, `src/app/match_report/io/web/widgets/print_match_sheet_widget.rs`, `templates/widgets/print-match-sheet.html`, `assets/static/css/widgets/print-match-sheet.css`, `src/app/match_report/routes.rs`, `src/app/teams/io/web/widgets/print_preview_widget.rs`

## Objectif

L'option « Feuille de rapport de match » : **une page A4**, deux colonnes —
avant le match, actions par joueur, après le match —, en un ou deux
exemplaires.

## Conception

- **Routes** `…/match-report/print/sheet/{team_id}` et `…/with/{other_team_id}`,
  query `home`, `round_id`.
- **Service**, par équipe : nom, roster, coach, VE courante (R4), fans,
  joueurs alignables journaliers compris (R6), barème SPP du roster.
- Statut financier par `top_dog` et `tv_gap` (602) : favori, ou outsider avec
  l'écart de VE (R11) ; à égalité, l'équipe à domicile, mon équipe pour une
  autre équipe (R10). Sans adversaire : colonne à remplir à la main.
- `round_id` vérifié par `find_round_context` dans la saison de mon équipe,
  sinon ignoré.
- Rappels du MVP et des fans, cases de météo : `find_match_reminders` (R9) ;
  absents → pas de rappel, ligne de météo libre.
- Gains et bonus de non-temporisation : règles codées dans kreek, imprimées
  telles quelles.
- Deux exemplaires : l'aperçu pose deux fois l'emplacement.

## Checklist

- [ ] Service, contrôleur, gabarit, feuille CSS inscrite au bundle
- [ ] Tests unitaires : favori / outsider selon les VE · égalité · sans adversaire · `round_id` d'une autre saison ignoré · rappels absents · barème par roster
- [ ] Tient sur une page A4 en portrait et en paysage
- [ ] `make check-arch`, `make lint`, `make test`
