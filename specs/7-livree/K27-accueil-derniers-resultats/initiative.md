---
id: K27
title: "Accueil — Derniers résultats — Spec index"
conception: docs/specs/accueil-derniers-resultats/
---

# K27 — Accueil — Derniers résultats — Spec index

Initiative créée à la migration vers le kanban de la spec factory (2026-11-19), à partir de la conception de la fonctionnalité `docs/specs/accueil-derniers-resultats/` (workflow feature). Ses cartes y sont listées dans les `08-cards.md`.

Conception : [`docs/specs/accueil-derniers-resultats/`](../../../docs/specs/accueil-derniers-resultats/)

## Tickets

- `277-resultats-view-bypass-space-member-port` — `resultats_view.rs` contourne le port ACL vers `spaces` (livrée à la migration)
- `278-competitions-latest-results-projection` — BC `competitions` — colonne `published_at` sur la projection résultats (livrée à la migration)
- `279-competitions-latest-results-repository` — BC `competitions` — lecture des derniers résultats d'un espace (livrée à la migration)
- `280-competitions-latest-results-authorization` — BC `competitions` — autorisation + VMs des derniers résultats (livrée à la migration)
- `281-competitions-latest-results-widget` — BC `competitions` — widget "Derniers résultats" (livrée à la migration)
- `282-news-home-latest-results-integration` — BC `news` — intégration du widget "Derniers résultats" sur l'accueil (livrée à la migration)
- `283-accueil-latest-results-e2e` — Tests E2E — widget "Derniers résultats" sur l'accueil (livrée à la migration)
