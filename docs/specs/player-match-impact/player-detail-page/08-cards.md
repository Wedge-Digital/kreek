# Player detail page — Phase 8 : Cartes kanban

9 cartes créées dans `kanban/ready_to_be_done/`, ordonnées par dépendance.

| # | Carte | Dépend de | Résumé |
|---|---|---|---|
| 160 | `shared-kernel-team-match-concluded-enriched` | — | `TeamMatchConcluded` enrichi (journée, adversaire, scores) |
| 161 | `players-domain-match-concluded` | — | Compteur `matches_played` + domain event `MatchConcluded` |
| 162 | `players-persistence-match-concluded` | 161 | Persistance `MatchConcluded` + `find_events_by_id` |
| 163 | `match-report-publisher-team-match-concluded-enriched` | 160 | Émission enrichie côté `match_report` |
| 164 | `players-listener-match-concluded-restructured` | 160, 161, 162, 163 | Listener restructuré (tous les joueurs, pas seulement MissingNextGame) |
| 165 | `players-match-history-service` | 161, 162 | Domain service de reconstruction de l'historique |
| 166 | `players-player-detail-page` | 156, 161, 162, 165 | Handler + template de la fiche joueur |
| 167 | `players-routing-player-detail` | 166 | Route + redirection du clic depuis le tableau roster |
| 168 | `players-player-detail-e2e` | 167 | Tests E2E Playwright (vraie surface UI cette fois) |

Groupes parallélisables : {160, 161} démarrables immédiatement. {162} après 161,
{163} après 160. {164} après 160+161+162+163. {165} après 161+162. {166} après
156(déjà fait)+161+162+165. {167} après 166. {168} en dernier.
