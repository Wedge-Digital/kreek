# Un conflit d'écriture se dit

**Épic :** E19 — Saisie du compte rendu : droits et robustesse · **Ordre :** 3 · **Dépend de :** —
**Conception :** `docs/specs/saisie-mobile/E19-cadrage.md` (R4)

## Objectif

Deux écritures simultanées sur un compte rendu sont déjà refusées par l'event
store (`RepositoryError::ConcurrentWrite`), mais le refus arrive en 500 : les
use cases le convertissent en `Repository(String)` et son type disparaît. Après
la carte : **409**, un message, et la page qui se recharge.

## Conception

- Une variante `Conflict` dans les erreurs des use cases d'écriture : fan
  factor, coups de pouce, action, suppression d'action, après-match, sélection,
  publication. `ConcurrentWrite` y est converti au lieu de `Repository`.
- Une conversion commune aux contrôleurs : `409`, `{ error: "Le match vient
  d'être modifié — la page se recharge." }`, et `HX-Refresh: true`.

## Données de production

Aucune. Le conflit est déjà refusé ; seule la réponse change.

## Checklist

- [ ] `Conflict` dans chaque erreur de use case d'écriture, alimenté par `ConcurrentWrite`
- [ ] La conversion commune, appelée par chaque contrôleur concerné
- [ ] Tests unitaires : chaque use case relaie `Conflict` ; la conversion rend 409 et `HX-Refresh`
- [ ] `make lint`, `make check-arch`, `make test`
