---
id: K30
title: "Competition Matchs — Spec index"
conception: docs/specs/competition-matchs/
---

# K30 — Competition Matchs — Spec index

Initiative créée à la migration vers le kanban de la spec factory (2026-11-19), à partir de la conception de la fonctionnalité `docs/specs/competition-matchs/` (workflow feature). Ses cartes y sont listées dans les `08-cards.md`.

Conception : [`docs/specs/competition-matchs/`](../../../docs/specs/competition-matchs/)

## Tickets

- `130-comp-matchs-enrich-events` — 130 — Enrichissement des events PairingCreated et MatchReportConfirmed (livrée à la migration)
- `131-comp-matchs-migration-projection` — 131 — Migration SQL : table competition_match_display_proj (livrée à la migration)
- `132-comp-matchs-pairing-listeners` — 132 — Listeners PairingCreated / PairingDeleted → projection (livrée à la migration)
- `133-comp-matchs-match-report-listener` — 133 — Listener MatchReportConfirmed → UPDATE in_progress (livrée à la migration)
- `134-comp-matchs-repository-queries` — 134 — Repository : list_resultats et list_calendrier (livrée à la migration)
- `135-comp-matchs-resultats-handler` — 135 — Handler + template onglet Résultats (livrée à la migration)
- `136-comp-matchs-calendrier-handler` — 136 — Handler + template onglet Calendrier (livrée à la migration)
- `137-comp-matchs-integration-page` — 137 — Intégration : mise à jour competition_detail + routes (livrée à la migration)
- `138-comp-matchs-e2e-tests` — 138 — Tests E2E : onglets Résultats et Calendrier (livrée à la migration)
