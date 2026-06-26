# Step 2.1 — Coups de pouce — Cartes kanban

| # | Titre | Dépend de |
|---|---|---|
| 105 | Domain — value objects + events inducements | 99, 100 |
| 106 | Domain — méthodes agrégat MatchReportPreMatch | 105 |
| 107 | Migration SQL + projection repository | 105 |
| 108 | Ports extension + infrastructure adapters | 105 |
| 109 | Use case record_fan_factor (extension TV + routing) | 106, 108 |
| 110 | Use case record_inducements (nouveau) | 106, 108 |
| 111 | BC References — widget inducement-selector | — |
| 112 | BC MatchReport — page inducements GET + POST | 109, 110, 111 |
| 113 | E2E tests step2-inducements | 112 |
