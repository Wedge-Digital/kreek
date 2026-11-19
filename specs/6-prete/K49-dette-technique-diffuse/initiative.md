---
id: K49
title: "Dette technique diffuse"
status: in_progress
---

# K49 — Dette technique diffuse

> **Rangement.** Redescendue de `6-prete/` en `5-tickets/` le 2026-11-19 à la relecture, puis remise en `6-prete/` le même jour sur décision de l'humain : la carte 14 est faite (`src/app/shared_kernel/identity/cloudinary.rs`, avec son test) et part en `8-abandonnee/` (K215) ; les cartes 09 et 11, aux chemins périmés, sont remplacées par les cartes 670 et 671, réécrites sur le code actuel, et rangées elles aussi dans K215.

Initiative créée à la migration vers le kanban de la spec factory (2026-11-19), à partir de le regroupement nommé par `kanban/epics/README.md` (section « Les cartes sans épic »), devenu `specs/README.md`.

## Tickets

- `670-entity-sans-eq-concurrente` — `Entity::eq()` fait concurrence à `PartialEq::eq` (remplace la 09, réécrite le 2026-11-19)
- `10-bypass-auth-appstate` — `bypass_auth` dans `AppState` (prête à la migration)
- `671-initials-une-seule-definition` — Fonction `initials()` recopiée dans treize fichiers (remplace la 11, réécrite le 2026-11-19)
- `15-url-generation-string-replace` — Génération d'URLs par string replacement (prête à la migration)
- `254-draft-team-erreur-json-trompeuse` — `team_creation` — Un corps de requête invalide accuse la compétition (prête à la migration)
