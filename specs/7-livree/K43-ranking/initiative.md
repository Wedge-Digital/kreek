---
id: K43
title: "Ranking — Spec index"
conception: docs/specs/ranking/
---

# K43 — Ranking — Spec index

Initiative créée à la migration vers le kanban de la spec factory (2026-11-19), à partir de la conception de la fonctionnalité `docs/specs/ranking/` (workflow feature). Ses cartes y sont listées dans les `08-cards.md`.

Conception : [`docs/specs/ranking/`](../../../docs/specs/ranking/)

## Tickets

- `192-ranking-domaine` — BC `ranking` — Domaine (RankingLine, calcul des points de classement) (livrée à la migration)
- `193-ranking-port-competitions` — BC `ranking` — Port ACL vers `competitions` (livrée à la migration)
- `194-ranking-repository` — BC `ranking` — Repository interne (`IRankingRepository` + migration) (livrée à la migration)
- `195-ranking-use-case-record-match` — BC `ranking` — Use case `record_match_ranking` (livrée à la migration)
- `196-ranking-listener-et-cablage` — BC `ranking` — Listener `MatchReportPublished` + câblage complet du BC (livrée à la migration)
- `197-ranking-widget-classement` — BC `ranking` — Widget Classement (handler + template + CSS) (livrée à la migration)
- `198-competitions-host-classement` — BC `competitions` — Onglet Classement devient un simple host du widget `ranking` (livrée à la migration)
- `208-tiebreak-catalogue-ranking` — Départages — Catalogue des critères dans le BC `ranking` (livrée à la migration)
- `209-tiebreak-acl-catalogue` — Départages — ACL du catalogue vers `competitions` (port + adapter) (livrée à la migration)
- `210-tiebreak-domaine-competitions` — Départages — Domaine `competitions` : VOs + erreurs (additif) (livrée à la migration)
- `211-tiebreak-cablage` — Départages — Câblage complet : agrégat, use case, handlers, formulaire (livrée à la migration)
- `212-tiebreak-e2e` — Départages — Tests E2E navigateur (livrée à la migration)
- `213-ranking-bonus-points-separes` — Ranking — Séparation des points bonus dans la ranking line (livrée à la migration)
- `214-tiebreak-acl-config` — Départages — Propagation ACL de la configuration vers `ranking` (livrée à la migration)
- `215-ranking-command-team-stats` — Ranking — Regroupement des stats de match par équipe (refacto pure) (livrée à la migration)
- `216-tiebreak-compteurs` — Départages — Compteurs cumulés sur la ranking line (livrée à la migration)
- `217-tiebreak-comparaison-domaine` — Départages — Comparaison et rangs dans le domaine (additif) (livrée à la migration)
- `218-tiebreak-cablage-lecture` — Départages — Câblage du classement ordonné (livrée à la migration)
- `219-tiebreak-e2e` — Départages — Tests E2E du classement ordonné (livrée à la migration)
- `220-detailed-standings-outcomes-domaine` — Classement détaillé — Résolution du critère décisif dans le domaine (livrée à la migration)
- `221-detailed-standings-refactos-preparatoires` — Classement détaillé — Refactos préparatoires (livrée à la migration)
- `222-detailed-standings-onglet` — Classement détaillé — Onglet complet, sans mise en évidence (livrée à la migration)
- `223-detailed-standings-mise-en-evidence` — Classement détaillé — Mise en évidence du critère décisif (livrée à la migration)
- `224-detailed-standings-e2e` — Classement détaillé — Tests E2E (livrée à la migration)
