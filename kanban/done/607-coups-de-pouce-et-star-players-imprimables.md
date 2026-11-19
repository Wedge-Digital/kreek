# Les coups de pouce et les star players disponibles

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 2
**Dépend de :** 605, 596, 594
**Spec :** `04-dtos.md` (`InducementsPrintVm`), `05-use-cases.md` (`inducements_print_service`), R7, R13
**Fichiers :** `src/app/match_report/use_cases/inducements_print_service.rs`, `src/app/match_report/io/web/widgets/print_inducements_widget.rs`, `templates/widgets/print-inducements.html`, `assets/static/css/widgets/print-inducements.css`, `src/app/match_report/routes.rs`, `src/app/teams/io/web/widgets/print_preview_widget.rs`

## Objectif

Les options « Coups de pouce disponibles » et « Star players disponibles » :
le catalogue du palier de chaque équipe, au prix de son roster, et les fiches
des star players qu'elle peut engager.

## Conception

- **Routes** `…/match-report/print/inducements/{team_id}` et `…/with/{other_team_id}`,
  query `inducements`, `star_players`, `orientation`, `density`.
- **Service**, par équipe : `find_team_info` (roster), `find_team_season`,
  `find_tier_rules_for_roster` — palier, prix et disponibilité, déjà calculés
  par `cout_pour_roster` et `est_disponible_pour`. Puis libellés et star players
  par `IReferenceCatalogPort`.
- Une ligne par coup de pouce, une colonne de prix par équipe ; « — » si
  indisponible (R7). Équipe sans saison ou sans palier : colonne vide signalée
  « palier introuvable ».
- Règles d'achat rappelées en texte (ordre, petite monnaie, deux star players
  au plus, mercenaires, journaliers) : elles sont codées dans kreek.
- Les deux options partagent leurs feuilles quand elles sont cochées ensemble.

## Checklist

- [x] Service, contrôleur, gabarit, feuille CSS inscrite au bundle
- [x] Tests unitaires : prix réduit pour un roster · « — » pour un coup de pouce réservé · palier introuvable · star player filtré par roster · union des uids de deux paliers
- [x] `make check-arch`, `make lint`, `make test`
