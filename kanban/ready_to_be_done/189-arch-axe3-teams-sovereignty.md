# Architecture — Souveraineté des données : BC `teams`

**Priorité : moyenne**
**Dépend de :** rien
**Contexte :** `teams` — io/web

## Objectif

`team_detail.rs` importe directement `IReferenceRepository` du BC `references`. Appliquer le pattern port+adapter déjà établi ailleurs dans le projet (ex. `team_creation → references` via `IReferenceDataPort`).

## Violations recensées (`make check-arch`, axe 3)

- `io/web/team_detail.rs:2` (`use crate::app::references::domain::port::IReferenceRepository`)
- `io/web/team_detail.rs:288` (`state.references.repository.as_ref()`)

## Action

1. Identifier précisément les données de `references` consommées par `team_detail.rs` (résolution de roster/skills pour l'affichage — à vérifier dans le code).
2. Définir `IRosterInfoPort` (ou nom équivalent) + DTOs minimaux dans `src/app/teams/ports.rs`.
3. Créer `src/infrastructure/teams/roster_info_adapter.rs`.
4. Injecter dans `TeamsContext` + `main.rs`.
5. Refactorer `team_detail.rs` pour consommer le port.

## Checklist

- [ ] Port `IRosterInfoPort` + DTOs dans `teams/ports.rs`
- [ ] Adapter `infrastructure/teams/roster_info_adapter.rs`
- [ ] Injection dans `TeamsContext` + `main.rs`
- [ ] `team_detail.rs` n'importe plus `crate::app::references`
- [ ] `cargo check` sans erreur
- [ ] `make check-arch` : axe 3 clean pour `teams`
- [ ] `cargo test` + `make e2e` passent
