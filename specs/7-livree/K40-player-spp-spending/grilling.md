# K40 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de `docs/specs/player-spp-spending/`. Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K40-B1** — BC `players` — Domaine : achat de compétences et augmentations de caractéristiques (ticket `175-players-domain-improvement`)
- **K40-B2** — BC `references` — Table de valeur des améliorations (value_delta) (ticket `176-references-improvement-value-data`)
- **K40-B3** — BC `players` — Port ACL `ISkillCatalogPort` vers `references` (ticket `177-players-skill-catalog-port`)
- **K40-B4** — BC `players` — Use case d'achat de compétence (ticket `178-players-purchase-skill-use-case`)
- **K40-B5** — BC `players` — Use case d'augmentation de caractéristique (ticket `179-players-increase-stat-use-case`)
- **K40-B6** — BC `players`/`teams` — App event d'achat → `team_value` (ticket `180-teams-player-improvement-app-event`)
- **K40-B7** — BC `players` — Slot unique sur la fiche joueur (journal / dépense SPP) (ticket `181-players-detail-page-widget-slot`)
- **K40-B8** — BC `players` — Widget de dépense de SPP (compétences + caractéristiques) (ticket `182-players-spp-spending-widget`)
- **K40-B9** — BC `players` — Tests E2E dépense de SPP (ticket `183-players-spp-spending-e2e`)

## Hors périmètre

Non tracé dans le dépôt.
