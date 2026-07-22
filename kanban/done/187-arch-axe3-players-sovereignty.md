# Architecture — Souveraineté des données : BC `players`

**Priorité : moyenne**
**Dépend de :** rien
**Contexte :** `players` — io/web, use_cases, app_events

## Objectif

Le BC `players` importe directement des types/repositories d'autres BCs (`references`, `teams`, `competitions`, `spaces`) au lieu de passer par des ports définis dans `players/ports.rs` + des adapters dans `src/infrastructure/players/`. Violation de la règle de souveraineté des données (CLAUDE.md).

Le pattern correct existe déjà partiellement : `src/infrastructure/players/skill_catalog_adapter.rs`. Étendre ce pattern aux autres besoins.

## Violations recensées (`make check-arch`, axe 3)

### `players → references`
- `context.rs:6`
- `io/web/player_table.rs:3,97`
- `io/web/player_detail_controller.rs:243`
- `io/web/widgets/spp_spending_widget.rs:176`
- `io/app_events/player_match_impact_listener.rs:5`
- `io/app_events/team_created_listener.rs:11`
- `use_cases/player_stats_service.rs:3`

### `players → teams`
- `io/web/increase_stat_controller.rs:8,37` (`GamePhase`, `team_repository.find_by_id`)
- `io/web/player_detail_controller.rs:11,214`
- `io/web/purchase_skill_controller.rs:8,41`
- `io/web/widgets/spp_spending_widget.rs:11,186`

### `players → competitions`
- `io/web/player_detail_controller.rs:119` (`competition_repository.find_base_info`)
- `io/web/purchase_skill_controller.rs:114`

### `players → spaces`
- `io/web/player_detail_controller.rs:109` (`space_repository.find_member_profile`)
- `io/web/purchase_skill_controller.rs:103`

## Action

1. Auditer chaque usage ci-dessus pour déterminer les DTOs minimaux réellement nécessaires (ne pas exposer plus que ce qui est consommé).
2. Définir dans `src/app/players/ports.rs` :
   - `IPlayerReferencePort` (skills de base, etc. — vérifier chevauchement avec l'existant `skill_catalog_adapter`/port déjà en place, réutiliser si possible plutôt que dupliquer)
   - `IPlayerRosterPort` (résoudre `GamePhase` d'une équipe + infos équipe nécessaires par player_id/team_id)
   - `IPlayerCompetitionPort` (`find_base_info` compétition)
   - `IPlayerSpaceMemberPort` (`find_member_profile`)
3. Créer les adapters correspondants dans `src/infrastructure/players/` (un fichier par port, convention déjà en place).
4. Injecter les nouveaux ports dans `PlayersContext` (`context.rs`) et dans `main.rs`.
5. Refactorer chaque fichier listé pour consommer les ports au lieu des imports directs.
6. Si la transformation DTO→domaine dépasse un simple passe-plat, créer un domain service dans `players/use_cases/` (cf. règle "Domain services pour données inter-BCs" du CLAUDE.md) plutôt que de laisser les handlers manipuler les DTOs du port.

## Checklist

- [ ] Ports définis dans `players/ports.rs` (réutilisation de l'existant vérifiée avant duplication)
- [ ] Adapters créés dans `infrastructure/players/`
- [ ] Injection dans `PlayersContext` + `main.rs`
- [ ] Tous les fichiers listés refactorés, plus aucun `use crate::app::references::`, `use crate::app::teams::`, `use crate::app::competitions::`, `use crate::app::spaces::` dans `players/` (hors `::routes::`)
- [ ] `cargo check` sans erreur
- [ ] `make check-arch` : axe 3 clean pour `players`
- [ ] `cargo test` + `make e2e` passent
