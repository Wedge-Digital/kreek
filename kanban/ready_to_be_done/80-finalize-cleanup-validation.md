# BC `team_creation` — Nettoyage et validation finale finalize-team

**Priorité : haute (dernière carte)**
**Dépend de :** `79-finalize-page-assembly.md`
**Contexte :** BC `team_creation` — nettoyage

## Objectif

Supprimer le code mort, vérifier `check-arch`, lancer les tests E2E.

---

## Checklist

- [ ] Supprimer `batch_finalize.rs` si encore présent (carte 77)
- [ ] Supprimer les structs mortes (`FinalizeData`, `PlayerJson`, `PricingJson`, `AssignmentRequest`)
- [ ] Supprimer les imports inutilisés
- [ ] `check-arch` — aucune violation pour `finalize_team.rs`
- [ ] `cargo check` — 0 erreur, pas de warning dans nos fichiers
- [ ] Tests unitaires passent (163+)
- [ ] Tests E2E passent (16+)
- [ ] Vérifier manuellement le parcours : sélection joueur → skill picker → spend SPP → cancel SPP → submit
