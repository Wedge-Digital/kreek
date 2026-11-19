# K49 — Grilling

> Reconstitué à la migration (2026-11-19), à partir des cartes elles-mêmes. Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K49-B1** — `Entity::eq()` shadowe `PartialEq::eq` (ticket `670-entity-sans-eq-concurrente`, qui remplace `09-entity-trait-eq`)
- **K49-B2** — `bypass_auth` dans `AppState` (ticket `10-bypass-auth-appstate`)
- **K49-B3** — Fonction `initials()` dupliquée (ticket `671-initials-une-seule-definition`, qui remplace `11-initials-dupliquee`)
- **K49-B5** — Génération d'URLs par string replacement (ticket `15-url-generation-string-replace`)
- **K49-B6** — `team_creation` — Un corps de requête invalide accuse la compétition (ticket `254-draft-team-erreur-json-trompeuse`)

## Hors périmètre

- `cloudinary_transform()` privée et non réutilisable (ancien besoin K49-B4, carte `14-cloudinary-transform-privee`) : déjà fait, la carte est abandonnée dans K215.
- La copie `initials_from` du domaine `teams` (voir la carte 671).
