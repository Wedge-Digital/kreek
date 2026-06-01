# BC `teams` — Structure et scaffolding

**Priorité : haute**
**Dépend de :** —
**Contexte :** `teams` (nouveau BC)

## Objectif

Créer la structure de dossiers et de modules du BC `teams`, à l'image du BC `team_creation`. Aucune logique métier dans cette carte — uniquement les fichiers squelettes.

---

## Conception

### Arborescence cible

```
src/app/teams/
├── mod.rs
├── context.rs                  # TeamsContext + init
├── router.rs                   # build_router()
├── routes.rs                   # Routes (générateur d'URLs)
├── domain/
│   ├── mod.rs
│   ├── team.rs                 # agrégat Team (carte 28)
│   └── error.rs                # DomainError
├── ports.rs                    # ITeamRepository
├── use_cases/
│   ├── mod.rs
│   └── commands.rs
├── io/
│   ├── mod.rs
│   ├── repository/
│   │   └── team_repository.rs  # implémentation sqlx (carte 29)
│   ├── app_events/
│   │   └── team_created_listener.rs  # consommateur TeamCreated (carte 31)
│   └── web/
│       ├── mod.rs
│       └── team_detail.rs      # handler fiche équipe (carte 34)
└── templates/
    └── team-detail.html        # template Askama (carte 34)
```

### Rattachement dans `src/app/mod.rs` et `main.rs`

Ajouter `pub mod teams;` dans `src/app/mod.rs`.

Instancier `TeamsContext::new(pool, app_event_bus)` dans `main.rs`.

---

## Checklist

- [ ] Créer l'arborescence `src/app/teams/` avec les fichiers `mod.rs` vides
- [ ] `context.rs` : struct `TeamsContext` squelette + `new()`
- [ ] `routes.rs` : struct `Routes` avec `team_detail()` (retourne une URL statique pour l'instant)
- [ ] `router.rs` : `build_router()` vide raccordé dans `main.rs`
- [ ] `domain/error.rs` : `DomainError` enum vide avec `thiserror`
- [ ] `ports.rs` : `ITeamRepository` trait vide
- [ ] Ajouter `pub mod teams` dans `src/app/mod.rs`
- [ ] Compiler sans erreur