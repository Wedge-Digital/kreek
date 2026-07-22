# Architecture — Souveraineté des données : BC `references` → `team_creation` (sens inverse)

**Priorité : moyenne**
**Dépend de :** rien
**Contexte :** `references` — io/repository ; `team_creation` — domain ; `shared_kernel`

## Objectif

`in_memory_reference_repository.rs` (BC `references`) importe `RosterName` depuis `team_creation::domain::roster`. C'est le sens inverse de toutes les autres violations : `references` est le BC le plus fondamental (données statiques consommées par tout le reste), il ne devrait dépendre d'aucun autre BC applicatif. `RosterName` n'a pas sa place dans `team_creation` — c'est un concept partagé par plusieurs BCs (`references`, `team_creation`, `teams` a déjà son propre `RosterName` dans `teams/domain/value_objects.rs` d'ailleurs, à vérifier s'il s'agit du même concept ou d'un doublon légitime).

## Violation recensée (`make check-arch`, axe 3)

- `io/repository/in_memory_reference_repository.rs:10` (`use crate::app::team_creation::domain::roster::RosterName`)

## Action

1. Vérifier l'usage exact de `RosterName` dans `in_memory_reference_repository.rs` (quel besoin précis).
2. Vérifier si `RosterName` existe déjà en double ailleurs (`teams/domain/value_objects.rs` a son propre `RosterName` d'après l'audit axe 3 — clarifier si c'est un concept identique dupliqué intentionnellement par BC, ou s'il faut converger).
3. Décision de modélisation : déplacer `RosterName` (ou une définition équivalente) dans `src/app/shared_kernel/` si le concept est légitimement partagé entre `references` et `team_creation` ; sinon, définir un type propre à `references` sans dépendre de `team_creation`.
4. Mettre à jour les imports dans `team_creation` et `references` en conséquence.

## Checklist

- [ ] Analyse de duplication `RosterName` (`teams` vs `team_creation` vs besoin de `references`) documentée
- [ ] Décision de modélisation actée (shared_kernel ou type propre à `references`)
- [ ] `in_memory_reference_repository.rs` n'importe plus `crate::app::team_creation`
- [ ] `cargo check` sans erreur
- [ ] `make check-arch` : axe 3 clean pour `references`
- [ ] `cargo test` + `make e2e` passent
