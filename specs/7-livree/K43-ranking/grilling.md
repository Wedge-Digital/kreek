# K43 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de `docs/specs/ranking/`. Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K43-B1** — BC `ranking` — Domaine (RankingLine, calcul des points de classement) (ticket `192-ranking-domaine`)
- **K43-B2** — BC `ranking` — Port ACL vers `competitions` (ticket `193-ranking-port-competitions`)
- **K43-B3** — BC `ranking` — Repository interne (`IRankingRepository` + migration) (ticket `194-ranking-repository`)
- **K43-B4** — BC `ranking` — Use case `record_match_ranking` (ticket `195-ranking-use-case-record-match`)
- **K43-B5** — BC `ranking` — Listener `MatchReportPublished` + câblage complet du BC (ticket `196-ranking-listener-et-cablage`)
- **K43-B6** — BC `ranking` — Widget Classement (handler + template + CSS) (ticket `197-ranking-widget-classement`)
- **K43-B7** — BC `competitions` — Onglet Classement devient un simple host du widget `ranking` (ticket `198-competitions-host-classement`)
- **K43-B8** — Départages — Catalogue des critères dans le BC `ranking` (ticket `208-tiebreak-catalogue-ranking`)
- **K43-B9** — Départages — ACL du catalogue vers `competitions` (port + adapter) (ticket `209-tiebreak-acl-catalogue`)
- **K43-B10** — Départages — Domaine `competitions` : VOs + erreurs (additif) (ticket `210-tiebreak-domaine-competitions`)
- **K43-B11** — Départages — Câblage complet : agrégat, use case, handlers, formulaire (ticket `211-tiebreak-cablage`)
- **K43-B12** — Départages — Tests E2E navigateur (ticket `212-tiebreak-e2e`)
- **K43-B13** — Ranking — Séparation des points bonus dans la ranking line (ticket `213-ranking-bonus-points-separes`)
- **K43-B14** — Départages — Propagation ACL de la configuration vers `ranking` (ticket `214-tiebreak-acl-config`)
- **K43-B15** — Ranking — Regroupement des stats de match par équipe (refacto pure) (ticket `215-ranking-command-team-stats`)
- **K43-B16** — Départages — Compteurs cumulés sur la ranking line (ticket `216-tiebreak-compteurs`)
- **K43-B17** — Départages — Comparaison et rangs dans le domaine (additif) (ticket `217-tiebreak-comparaison-domaine`)
- **K43-B18** — Départages — Câblage du classement ordonné (ticket `218-tiebreak-cablage-lecture`)
- **K43-B19** — Départages — Tests E2E du classement ordonné (ticket `219-tiebreak-e2e`)
- **K43-B20** — Classement détaillé — Résolution du critère décisif dans le domaine (ticket `220-detailed-standings-outcomes-domaine`)
- **K43-B21** — Classement détaillé — Refactos préparatoires (ticket `221-detailed-standings-refactos-preparatoires`)
- **K43-B22** — Classement détaillé — Onglet complet, sans mise en évidence (ticket `222-detailed-standings-onglet`)
- **K43-B23** — Classement détaillé — Mise en évidence du critère décisif (ticket `223-detailed-standings-mise-en-evidence`)
- **K43-B24** — Classement détaillé — Tests E2E (ticket `224-detailed-standings-e2e`)

## Hors périmètre

Non tracé dans le dépôt.
