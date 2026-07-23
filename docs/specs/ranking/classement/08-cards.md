# Classement — Phase 8 : Cartes kanban

8 cartes, ordonnées par dépendance (domaine → ports/repository → use case → câblage → widget → host → e2e). Toutes créées dans `kanban/ready_to_be_done/`.

| # | Carte | Résumé |
|---|---|---|
| [192](../../../../kanban/ready_to_be_done/192-ranking-domaine.md) | Domaine | `RankingLine`, `MatchOutcome`, VOs (`MatchScore`, `RankingPoints`), `record_match`/`derive_outcome`, tests unitaires |
| [193](../../../../kanban/ready_to_be_done/193-ranking-port-competitions.md) | Port ACL competitions | `IRankingCompetitionPort` (règles + équipes inscrites) + adapter, réutilise `ITeamInfoPort` existant |
| [194](../../../../kanban/ready_to_be_done/194-ranking-repository.md) | Repository interne | Table `ranking_lines` (avec `sequence`), `IRankingRepository`, implémentation Postgres |
| [195](../../../../kanban/ready_to_be_done/195-ranking-use-case-record-match.md) | Use case | `record_match_ranking_use_case` — orchestration pure, testable avec doublures |
| [196](../../../../kanban/ready_to_be_done/196-ranking-listener-et-cablage.md) | Listener + câblage | Écoute `MatchReportPublished`, `RankingContext`, enregistrement `AppState`/`main.rs` |
| [197](../../../../kanban/ready_to_be_done/197-ranking-widget-classement.md) | Widget | Handler + template (4 états) + CSS autonome + route |
| [198](../../../../kanban/ready_to_be_done/198-competitions-host-classement.md) | Host `competitions` | Suppression du mock, onglet devient un simple host `hx-get` |
| [199](../../../../kanban/ready_to_be_done/199-ranking-e2e.md) | Tests E2E | 6 scénarios Playwright couvrant les 4 états + le cumul multi-matchs |

Chaque carte est réalisable en une session, compilable/testable/commitable indépendamment (sauf 196 qui est le point de câblage complet du BC).
