# Onglets Résultats & Calendrier — Phase 8 : Cartes kanban

## Ordre d'implémentation

| # | Carte | Dépendances |
|---|---|---|
| 130 | Enrichissement events `PairingCreated` + `MatchReportConfirmed` | — |
| 131 | Migration SQL : table `competition_match_display_proj` | 130 |
| 132 | Listeners `PairingCreated` / `PairingDeleted` → projection | 130, 131 |
| 133 | Listener `MatchReportConfirmed` → UPDATE `in_progress` | 130, 131 |
| 134 | Repository : `list_resultats` + `list_calendrier` | 131 |
| 135 | Handler + template onglet Résultats | 134 |
| 136 | Handler + template onglet Calendrier | 134 |
| 137 | Intégration : mise à jour `competition_detail` + routes | 135, 136 |
| 138 | Tests E2E | 137 |

## Parallélisations possibles

- 132 et 133 sont indépendantes l'une de l'autre → peuvent être faites en parallèle
- 135 et 136 sont indépendantes l'une de l'autre → peuvent être faites en parallèle
