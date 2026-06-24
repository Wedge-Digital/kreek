# BC match_report — BC wiring (context + router)

**Priorité : haute**
**Dépend de :** 89, 90
**Contexte :** match_report step1, câblage du BC

## Objectif

Créer la structure du BC match_report : context, router, routes, modules, et intégrer dans `main.rs` et `AppState`.

## Conception

### Fichiers

```
src/app/match_report/
├── mod.rs
├── context.rs       ← MatchReportContext
├── routes.rs        ← Routes struct + path constants
└── router.rs        ← fn build_router() → Axum Router
```

### MatchReportContext

```rust
pub struct MatchReportContext {
    pub match_report_repo: Arc<dyn IMatchReportRepository>,
    pub competition_data: Arc<dyn ICompetitionDataPort>,
    pub team_data: Arc<dyn ITeamDataPort>,
    pub event_bus: EventBus,
}
```

### Intégration main.rs

- Instancier les adapters (`CompetitionDataAdapter`, `TeamDataAdapter`)
- Instancier `MatchReportContext`
- Ajouter au `AppState`
- Merger le router dans l'app

### Routes

```rust
pub const MATCH_REPORT_NEW: &str = "/app/{space_id}/match-report/new";
pub const MATCH_REPORT_EDIT: &str = "/app/{space_id}/match-report/{match_report_id}";
pub const MATCH_REPORT_SEASONS: &str = "/app/{space_id}/match-report/new/seasons";
pub const MATCH_REPORT_ROUNDS: &str = "/app/{space_id}/match-report/new/rounds";
pub const MATCH_REPORT_TEAMS: &str = "/app/{space_id}/match-report/new/teams";
```

Ajouter les routes dans `AppRoutes` pour un accès cross-BC.

## Checklist

- [ ] `mod.rs` : déclaration des sous-modules
- [ ] `context.rs` : `MatchReportContext` avec tous les ports + repo
- [ ] `routes.rs` : constantes + struct `Routes` avec méthodes de génération d'URL
- [ ] `router.rs` : `build_router()` vide (handlers branchés dans les cartes suivantes)
- [ ] Intégration `main.rs` : instanciation adapters, context, router merge
- [ ] Ajout dans `AppRoutes` (`src/app/routes.rs`)
- [ ] `cargo check` passe
