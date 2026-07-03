# Récap — Cartes kanban

## Ordre d'implémentation

| # | Carte | Dépendances |
|---|---|---|
| 144 | MR-RECAP-01 — Domaine MatchReportPublished | aucune |
| 145 | MR-RECAP-02 — Mini BC spp_calculator (stub) | aucune |
| 146 | MR-RECAP-03 — Ports & adapters | 145 |
| 147 | MR-RECAP-04 — Bus interne + publisher + AppEvent | 144 |
| 148 | MR-RECAP-05 — Use case publish_match_report_use_case | 144, 147 |
| 149 | MR-RECAP-06 — Handler + template + VMs | 144, 146, 148 |
| 150 | MR-RECAP-07 — Tests E2E | 149 |

144 et 145 peuvent démarrer en parallèle (aucune dépendance mutuelle).
