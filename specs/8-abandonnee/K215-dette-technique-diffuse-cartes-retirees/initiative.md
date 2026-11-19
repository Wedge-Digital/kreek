---
id: K215
title: "Dette technique diffuse — cartes retirées"
---

# K215 — Dette technique diffuse — cartes retirées

Initiative créée le 2026-11-19, à la reprise du rangement, pour garder trace des cartes
retirées de K49 (`6-prete/K49-dette-technique-diffuse/`), sur décision de l'humain.

## Tickets

- `14-cloudinary-transform-privee` — `cloudinary_transform()` privée et non réutilisable
  (abandonnée : déjà faite, `src/app/shared_kernel/identity/cloudinary.rs` et son test du
  cas sans `/upload/`)
- `09-entity-trait-eq` — `Entity::eq()` shadowe `PartialEq::eq` (abandonnée : chemins
  périmés, remplacée par `670-entity-sans-eq-concurrente` dans K49)
- `11-initials-dupliquee` — Fonction `initials()` dupliquée (abandonnée : chemins périmés,
  remplacée par `671-initials-une-seule-definition` dans K49)
