---
id: K49
title: "Dette technique diffuse"
status: in_progress
---

# K49 — Dette technique diffuse

> **Rangement.** Redescendue de `6-prete/` en `5-tickets/` le 2026-11-19, à la relecture : les cartes 09 et 11 citent des fichiers déplacés depuis (`Entity` vit dans `shared_kernel/identity/ids.rs` ; `initials` est dupliquée dans une dizaine de fichiers, pas deux), et la 14 semble faite (`shared_kernel/identity/cloudinary.rs` existe). Les cartes sont à relire avant de repasser en `6-prete/`.

Initiative créée à la migration vers le kanban de la spec factory (2026-11-19), à partir de le regroupement nommé par `kanban/epics/README.md` (section « Les cartes sans épic »), devenu `specs/README.md`.

## Tickets

- `09-entity-trait-eq` — `Entity::eq()` shadowe `PartialEq::eq` (prête à la migration)
- `10-bypass-auth-appstate` — `bypass_auth` dans `AppState` (prête à la migration)
- `11-initials-dupliquee` — Fonction `initials()` dupliquée (prête à la migration)
- `14-cloudinary-transform-privee` — `cloudinary_transform()` privée et non réutilisable (prête à la migration)
- `15-url-generation-string-replace` — Génération d'URLs par string replacement (prête à la migration)
- `254-draft-team-erreur-json-trompeuse` — `team_creation` — Un corps de requête invalide accuse la compétition (prête à la migration)
