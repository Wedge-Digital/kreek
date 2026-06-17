# BC `team_creation` — Simplification du POST finalize : submit seul

**Priorité : haute**
**Dépend de :** `76-finalize-acl-references.md`, `58-tc-finalization-spp.md`
**Contexte :** BC `team_creation` — handler finalize_team POST

## Objectif

Les SPP sont maintenant persistés individuellement via les endpoints spend/cancel (carte 58). Le POST de finalize ne reçoit plus d'assignments — il ne fait que valider et soumettre l'équipe.

---

## Situation actuelle

`post_finalize_team()` :
1. Reçoit `Vec<AssignmentRequest>` (skill assignments du front Alpine)
2. Pour chaque assignment, résout le coût via le repository references
3. Appelle `batch_finalize::execute()` qui applique les skills puis soumet
4. Retourne un `HX-Redirect` ou des erreurs

Ce modèle batch est obsolète : les skills sont déjà persistées par `spend_creation_spp` (carte 58).

---

## Plan

### Réécrire `post_finalize_team()`

Le handler ne reçoit plus de body. Il fait :
1. Charger l'équipe
2. Appeler `submit_team::execute()` (validation + soumission + event)
3. Retourner `HX-Redirect` ou erreurs de validation

C'est le même comportement que `submit_team` dans `build_team.rs`, mais pour la finalisation.

### Supprimer le code mort

- `AssignmentRequest` struct
- `batch_finalize.rs` use case (plus utilisé)
- Les imports associés

---

## Situation finale

- `post_finalize_team` est un simple appel à `submit_team::execute()`
- `batch_finalize.rs` est supprimé
- Le POST ne prend plus de body JSON

---

## Checklist

- [ ] Réécrire `post_finalize_team()` : appel direct à `submit_team::execute()`
- [ ] Supprimer `AssignmentRequest`
- [ ] Supprimer `batch_finalize.rs` et son module
- [ ] Mettre à jour les imports dans `finalize_team.rs`
- [ ] `cargo check` — 0 erreur
