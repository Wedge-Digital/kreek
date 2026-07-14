# Player report events — Phase 8 : Cartes kanban

8 cartes créées dans `kanban/ready_to_be_done/`, ordonnées par dépendance.

| # | Carte | Dépend de | Résumé |
|---|---|---|---|
| 152 | `references-spp-scale` | — | 5 méthodes barème SPP sur `IReferenceRepository` |
| 153 | `shared-kernel-player-match-impact-events` | — | Contrats `PlayerMatchImpactAppEvent` (Phase 4) |
| 154 | `players-domain-match-impact` | — | Agrégat `Player` étendu, 8 domain events, méthodes de commande, `apply()` |
| 155 | `players-persistence-match-impact` | 154 | `players_events`/`players_proj` étendus, migration `participation_status` |
| 156 | `players-stats-resolution-service` | 154 | Domain service `resolve_stats()` (base + `stat_adjustments`) |
| 157 | `match-report-publisher-player-events` | 153 | Publisher `match_report` étendu, émission par action + `TeamMatchConcluded` |
| 158 | `players-match-impact-listeners` | 152, 153, 154, 155 | Deux listeners `players` : impact par action + restauration `MissingNextGame` |
| 159 | `player-match-impact-integration-test` | 157, 158 | Test d'intégration bout-en-bout (remplace l'E2E Playwright, absent car pas de front) |

Groupes parallélisables : {152, 153, 154} peuvent démarrer immédiatement et en
parallèle. {155, 156} après 154. {157} après 153. {158} après 152+153+154+155.
{159} en dernier.
