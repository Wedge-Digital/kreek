# K30 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de `docs/specs/competition-matchs/`. Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K30-B1** — 130 — Enrichissement des events PairingCreated et MatchReportConfirmed (ticket `130-comp-matchs-enrich-events`)
- **K30-B2** — 131 — Migration SQL : table competition_match_display_proj (ticket `131-comp-matchs-migration-projection`)
- **K30-B3** — 132 — Listeners PairingCreated / PairingDeleted → projection (ticket `132-comp-matchs-pairing-listeners`)
- **K30-B4** — 133 — Listener MatchReportConfirmed → UPDATE in_progress (ticket `133-comp-matchs-match-report-listener`)
- **K30-B5** — 134 — Repository : list_resultats et list_calendrier (ticket `134-comp-matchs-repository-queries`)
- **K30-B6** — 135 — Handler + template onglet Résultats (ticket `135-comp-matchs-resultats-handler`)
- **K30-B7** — 136 — Handler + template onglet Calendrier (ticket `136-comp-matchs-calendrier-handler`)
- **K30-B8** — 137 — Intégration : mise à jour competition_detail + routes (ticket `137-comp-matchs-integration-page`)
- **K30-B9** — 138 — Tests E2E : onglets Résultats et Calendrier (ticket `138-comp-matchs-e2e-tests`)

## Hors périmètre

Non tracé dans le dépôt.
