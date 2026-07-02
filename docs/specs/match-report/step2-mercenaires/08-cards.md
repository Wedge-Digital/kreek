# Step 2 — Mercenaires — Cartes kanban

## Ordre d'implémentation

| # | Carte | Dépend de |
|---|-------|-----------|
| 124 | Domaine BR4 — validate_mercenary_limit + DomainError + tests | — |
| 125 | Ports + Infrastructure adapters (find_roster_positions, find_player_counts_by_position) | 124 |
| 126 | MercenaryLevel + extension record_inducements_use_case + collect_mercs | 125 |
| 127 | Route + widget GET mercenary_selector_widget | 125 |
| 128 | inducements_controller extension + inducements.html + tab bar migration | 126, 127 |
| 129 | E2E tests step2-mercenaires | 128 |

## Fichiers de spec

| Phase | Fichier |
|-------|---------|
| Front | `docs/specs/match-report/step2-mercenaires/02-front.md` |
| Back | `docs/specs/match-report/step2-mercenaires/03-back.md` |
| DTOs | `docs/specs/match-report/step2-mercenaires/04-dtos.md` |
| Use cases | `docs/specs/match-report/step2-mercenaires/05-use-cases.md` |
| Domaine | `docs/specs/match-report/step2-mercenaires/06-domaine.md` |
| Intégration | `docs/specs/match-report/step2-mercenaires/07-integration.md` |
