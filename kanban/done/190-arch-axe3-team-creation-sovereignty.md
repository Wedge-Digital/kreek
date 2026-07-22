# Architecture — Souveraineté des données : BC `team_creation` → `competitions`

**Priorité : moyenne**
**Dépend de :** rien
**Contexte :** `team_creation` — io/web

## Objectif

`finalize_team.rs` et `build_team/submit_team.rs` appellent directement `state.competitions.competition_repository.find_base_info(...)` et `state.competitions.season_repository.find_base_info(...)`. Un pattern similaire existe déjà : `src/infrastructure/team_creation/competition_rules_adapter.rs` — vérifier s'il couvre déjà `find_base_info` ou s'il faut l'étendre/compléter.

## Violations recensées (`make check-arch`, axe 3)

- `io/web/finalize_team.rs:110,115,278,283`
- `io/web/build_team/submit_team.rs:51,56`

## Action

1. Lire `src/infrastructure/team_creation/competition_rules_adapter.rs` et le port associé dans `team_creation/ports.rs` — déterminer s'il peut être étendu pour couvrir `find_base_info` (compétition + saison) plutôt que de créer un port parallèle.
2. Ajouter les méthodes manquantes au port existant (ou en créer un nouveau si le domaine couvert est vraiment différent) + à l'adapter.
3. Refactorer `finalize_team.rs` et `submit_team.rs` pour utiliser le port au lieu de `state.competitions`.

## Checklist

- [ ] Port étendu ou créé dans `team_creation/ports.rs`
- [ ] Adapter mis à jour dans `infrastructure/team_creation/`
- [ ] `finalize_team.rs` et `submit_team.rs` n'accèdent plus à `state.competitions`
- [ ] `cargo check` sans erreur
- [ ] `make check-arch` : axe 3 clean pour `team_creation`
- [ ] `cargo test` + `make e2e` passent
