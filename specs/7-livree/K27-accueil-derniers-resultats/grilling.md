# K27 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de `docs/specs/accueil-derniers-resultats/`. Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K27-B1** — `resultats_view.rs` contourne le port ACL vers `spaces` (ticket `277-resultats-view-bypass-space-member-port`)
- **K27-B2** — BC `competitions` — colonne `published_at` sur la projection résultats (ticket `278-competitions-latest-results-projection`)
- **K27-B3** — BC `competitions` — lecture des derniers résultats d'un espace (ticket `279-competitions-latest-results-repository`)
- **K27-B4** — BC `competitions` — autorisation + VMs des derniers résultats (ticket `280-competitions-latest-results-authorization`)
- **K27-B5** — BC `competitions` — widget "Derniers résultats" (ticket `281-competitions-latest-results-widget`)
- **K27-B6** — BC `news` — intégration du widget "Derniers résultats" sur l'accueil (ticket `282-news-home-latest-results-integration`)
- **K27-B7** — Tests E2E — widget "Derniers résultats" sur l'accueil (ticket `283-accueil-latest-results-e2e`)

## Hors périmètre

Non tracé dans le dépôt.
