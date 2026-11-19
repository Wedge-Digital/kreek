# K39 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de `docs/specs/player-match-impact/`. Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K39-B1** — BC `references` — Barème SPP par type d'action (ticket `152-references-spp-scale`)
- **K39-B2** — Shared kernel — App events `PlayerMatchImpactAppEvent` (ticket `153-shared-kernel-player-match-impact-events`)
- **K39-B3** — BC `players` — Domaine : impact des rapports de match sur `Player` (ticket `154-players-domain-match-impact`)
- **K39-B4** — BC `players` — Persistance des événements d'impact de match (ticket `155-players-persistence-match-impact`)
- **K39-B5** — BC `players` — Domain service : résolution des stats finales (ticket `156-players-stats-resolution-service`)
- **K39-B6** — BC `match_report` — Publisher étendu : émission des `PlayerReportEvents` (ticket `157-match-report-publisher-player-events`)
- **K39-B7** — BC `players` — Listeners `PlayerReportEvents` + `TeamMatchConcluded` (ticket `158-players-match-impact-listeners`)
- **K39-B8** — Player match impact — Test d'intégration bout-en-bout (ticket `159-player-match-impact-integration-test`)
- **K39-B9** — Shared kernel — `TeamMatchConcluded` enrichi (ticket `160-shared-kernel-team-match-concluded-enriched`)
- **K39-B10** — BC `players` — Domaine : compteur de matchs joués + event `MatchConcluded` (ticket `161-players-domain-match-concluded`)
- **K39-B11** — BC `players` — Persistance de `MatchConcluded` + lecture des events bruts (ticket `162-players-persistence-match-concluded`)
- **K39-B12** — BC `match_report` — Publisher : émission enrichie de `TeamMatchConcluded` (ticket `163-match-report-publisher-team-match-concluded-enriched`)
- **K39-B13** — BC `players` — Listener `TeamMatchConcluded` restructuré (ticket `164-players-listener-match-concluded-restructured`)
- **K39-B14** — BC `players` — Use case : reconstruction de l'historique de matchs (ticket `165-players-match-history-service`)
- **K39-B15** — BC `players` — Page de détail joueur (handler + template) (ticket `166-players-player-detail-page`)
- **K39-B16** — BC `players` — Routing de la fiche joueur (ticket `167-players-routing-player-detail`)
- **K39-B17** — Tests E2E — Fiche joueur (ticket `168-players-player-detail-e2e`)

## Hors périmètre

Non tracé dans le dépôt.
