# Step 3 & 4 — Actions match — Cartes kanban

| # | Titre | Dépend de |
|---|---|---|
| 114 | Domain — VOs, events, DomainErrors step3-4 | 113 |
| 115 | Domain — champs agrégat, méthodes + rehydratation | 114 |
| 116 | Migration SQL + projection repository step3-4 | 114 |
| 117 | Ports + adapters — IPlayerDataPort, find_journalier_position | 114 |
| 118 | Use case — init_temp_players + extension record_inducements | 115, 117 |
| 119 | Use cases — record_action + delete_action | 115, 117 |
| 120 | BC Players — widget match-player-selector | — |
| 121 | BC MatchReport — page hôte step3/step4 + turn-selector + temp-player-selector | 115, 116, 117, 118, 120 |
| 122 | BC MatchReport — action-panel + action-log + record_action_controller | 119, 121 |
| 123 | E2E tests step3-4-actions | 122 |
