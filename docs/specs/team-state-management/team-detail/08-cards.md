# Team detail — Phase 8 : Cartes kanban

5 cartes créées dans `kanban/ready_to_be_done/`, ordonnées par dépendance.

| # | Carte | Dépend de | Résumé |
|---|---|---|---|
| 169 | `teams-domain-dismissals-target-match-report-id` | — | `DismissalsPhaseValidated → ReadyToPlay` + champ `current_match_report_id` |
| 170 | `teams-validate-phase-use-cases` | 169 | 3 use cases + routes + handlers de validation de phase |
| 171 | `teams-team-detail-state-banner` | 169, 170 | `BannerVm` + bloc template + CSS |
| 172 | `teams-match-report-published-listener` | 169 | Câblage minimal `MatchReportPublished → PlayerImprovement` |
| 173 | `teams-team-detail-banner-e2e` | 171, 172 | Tests E2E des 7 scénarios de bandeau |

Groupes parallélisables : {169} démarre immédiatement. {170} et {172} après 169
(parallélisables entre eux). {171} après 169+170. {173} en dernier, après 171+172.
