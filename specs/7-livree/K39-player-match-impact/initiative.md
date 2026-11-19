---
id: K39
title: "Player match impact — Spec index"
conception: docs/specs/player-match-impact/
---

# K39 — Player match impact — Spec index

Initiative créée à la migration vers le kanban de la spec factory (2026-11-19), à partir de la conception de la fonctionnalité `docs/specs/player-match-impact/` (workflow feature). Ses cartes y sont listées dans les `08-cards.md`.

Conception : [`docs/specs/player-match-impact/`](../../../docs/specs/player-match-impact/)

## Tickets

- `152-references-spp-scale` — BC `references` — Barème SPP par type d'action (livrée à la migration)
- `153-shared-kernel-player-match-impact-events` — Shared kernel — App events `PlayerMatchImpactAppEvent` (livrée à la migration)
- `154-players-domain-match-impact` — BC `players` — Domaine : impact des rapports de match sur `Player` (livrée à la migration)
- `155-players-persistence-match-impact` — BC `players` — Persistance des événements d'impact de match (livrée à la migration)
- `156-players-stats-resolution-service` — BC `players` — Domain service : résolution des stats finales (livrée à la migration)
- `157-match-report-publisher-player-events` — BC `match_report` — Publisher étendu : émission des `PlayerReportEvents` (livrée à la migration)
- `158-players-match-impact-listeners` — BC `players` — Listeners `PlayerReportEvents` + `TeamMatchConcluded` (livrée à la migration)
- `159-player-match-impact-integration-test` — Player match impact — Test d'intégration bout-en-bout (livrée à la migration)
- `160-shared-kernel-team-match-concluded-enriched` — Shared kernel — `TeamMatchConcluded` enrichi (livrée à la migration)
- `161-players-domain-match-concluded` — BC `players` — Domaine : compteur de matchs joués + event `MatchConcluded` (livrée à la migration)
- `162-players-persistence-match-concluded` — BC `players` — Persistance de `MatchConcluded` + lecture des events bruts (livrée à la migration)
- `163-match-report-publisher-team-match-concluded-enriched` — BC `match_report` — Publisher : émission enrichie de `TeamMatchConcluded` (livrée à la migration)
- `164-players-listener-match-concluded-restructured` — BC `players` — Listener `TeamMatchConcluded` restructuré (livrée à la migration)
- `165-players-match-history-service` — BC `players` — Use case : reconstruction de l'historique de matchs (livrée à la migration)
- `166-players-player-detail-page` — BC `players` — Page de détail joueur (handler + template) (livrée à la migration)
- `167-players-routing-player-detail` — BC `players` — Routing de la fiche joueur (livrée à la migration)
- `168-players-player-detail-e2e` — Tests E2E — Fiche joueur (livrée à la migration)
