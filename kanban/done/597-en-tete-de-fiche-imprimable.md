# L'en-tête de fiche imprimable

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 1
**Dépend de :** 596
**Spec :** `04-dtos.md` (`TeamSheetHeadVm`, `StaffPrintLineVm`), `05-use-cases.md`, règles R4, R8, R14
**Fichiers :** `src/app/teams/use_cases/team_sheet_head_service.rs`, `src/app/teams/io/web/builders.rs`, `src/app/teams/io/web/widgets/print_sheet_head_widget.rs`, `templates/widgets/print-sheet-head.html`, `assets/static/css/widgets/print-sheet-head.css`, `src/app/teams/routes.rs`

## Objectif

Le haut de chaque fiche : identité, chiffres, staff.

## Conception

- **Route** `…/teams/{team_id}/print/sheet-head`, query `role` (`own` | `other`), `treasury`.
- **Service** `team_sheet_head_service::load` : `find_by_id`,
  `IRosterCatalogPort::find_catalog`, `ISquadPort::find_squad`.
- **VM** (`builders::team_sheet_head`) :
  - VE : la valeur persistée `team.team_value` (R4) — jamais recalculée ;
  - alignables : `presence.alignable()` (R14) ;
  - trésorerie : sur toutes les fiches, sans option (R8, révisée au test manuel) ;
  - staff : prix de la relance **et** du staff lus dans le catalogue du roster.
    La fiche, elle, code encore 50 / 10 / 10 en dur : écart noté hors périmètre
    dans le README de la spec, à ne pas corriger ici.
- Logo : le vrai logo (Cloudinary) ou les initiales — aucun `style="…"`.

## Checklist

- [x] Service, builder, contrôleur, gabarit, feuille CSS inscrite au bundle
- [x] Tests unitaires du builder : trésorerie sur toutes les fiches · prix du staff pris au catalogue · alignables selon `presence`
- [x] L'aperçu (596) appelle ce fragment
- [x] `make check-arch`, `make lint`, `make test`
