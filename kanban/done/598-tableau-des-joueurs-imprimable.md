# Le tableau des joueurs imprimable

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 1
**Dépend de :** 596
**Spec :** `04-dtos.md` (`PlayerPrintTableVm`, `PlayerPrintRowVm`), règles R5, R6, R14, R15
**Fichiers :** `src/app/players/io/web/widgets/player_table_widget.rs` (`build_player_rows`), `src/app/players/use_cases/player_print_service.rs`, `src/app/players/io/web/widgets/player_print_table_widget.rs`, `templates/widgets/print-player-table.html`, `assets/static/css/widgets/print-player-table.css`, `src/app/players/routes.rs`

## Objectif

Les seize lignes de la fiche, en lecture seule.

## Conception

- **Route** `…/players/by-team/{team_id}/print`, query `unavailable`, `blank_rows`, `density`.
- `build_player_rows` transmet `membership` : `is_journeyman` (R6).
- **Gabarit distinct** de `player-table-fragment.html`, qui embarque le
  formulaire d'édition.
- Colonnes : n°, nom, poste, MV/FO/AG/PA/AR (deltas résolus, séquelles
  comprises), compétences de base puis acquises en gras, blessures
  persistantes (compteur), **SPP restants** (R5), valeur.
- Indisponibles (`absence`) : afficher · barrer · masquer.
- Lignes vides : 16 − lignes imprimées (R15).
- Pas de soulignement des caractéristiques réduites : la projection ne
  distingue pas la part des séquelles (`03-back.md`).

## Checklist

- [x] `membership` jusqu'au VM, sans régression du widget existant
- [x] Service, contrôleur, gabarit, feuille CSS inscrite au bundle
- [x] Tests unitaires : lignes vides selon le nombre imprimé · masquer ajoute des lignes vides · journalier marqué · SPP = solde
- [x] L'aperçu (596) appelle ce fragment
- [x] `make check-arch`, `make lint`, `make test`
