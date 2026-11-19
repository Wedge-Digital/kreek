# Architecture — Souveraineté des données : BC `competitions`

**Priorité : moyenne**
**Dépend de :** rien
**Contexte :** `competitions` — io/web/admin

## Objectif

Étendre le pattern port+adapter déjà utilisé (`src/infrastructure/competitions/team_info_adapter.rs`) aux deux violations restantes du BC `competitions`.

## Violations recensées (`make check-arch`, axe 3)

### `competitions → references`
- `io/web/admin/summary_tab.rs:8` (`use ... references::domain::port::IReferenceRepository`), `:92` (`state.references.repository.as_ref()`)
- `io/web/admin/admin_page.rs:179` (`state.references.repository.as_ref()`)

### `competitions → spaces`
- `io/web/admin/admin_page.rs:77` (`state.spaces.space_repository.find_member_profile(&user.id, &space_entity_id)`)

## Action

1. Définir dans `src/app/competitions/ports.rs` :
   - un port pour les données de référence utilisées par `summary_tab.rs`/`admin_page.rs` (déterminer le besoin exact — probablement résolution de noms/libellés)
   - un port pour la résolution du profil membre (`find_member_profile`) — vérifier si un port équivalent existe déjà ailleurs dans le projet pour `spaces` (ex. `ISpaceUserCacheRepository` utilisé par `match_report`, cf. `src/infrastructure/match_report/coach_data_adapter.rs`) et le réutiliser/adapter plutôt que d'en recréer un nouveau
2. Créer le ou les adapters dans `src/infrastructure/competitions/`.
3. Injecter dans `CompetitionsContext` + `main.rs`.
4. Refactorer `summary_tab.rs` et `admin_page.rs`.

## Checklist

- [ ] Port(s) définis dans `competitions/ports.rs`
- [ ] Adapter(s) créés dans `infrastructure/competitions/`
- [ ] Injection dans `CompetitionsContext` + `main.rs`
- [ ] Plus aucun `use crate::app::references::` ni `use crate::app::spaces::` / `state.references` / `state.spaces` dans `competitions/` (hors `::routes::`)
- [ ] `cargo check` sans erreur
- [ ] `make check-arch` : axe 3 clean pour `competitions`
- [ ] `cargo test` + `make e2e` passent
