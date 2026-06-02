# Publication de l'app event `TeamCreated` depuis le BC `team_creation`

**Priorité : haute**
**Dépend de :** `26-submit-team.md`
**Contexte :** `team_creation` (émetteur) → app event bus

## Objectif

Ajouter un publisher dans le BC `team_creation` qui écoute le domain event `TeamSubmitted` et le traduit en app event `TeamCreated` sur l'app event bus, pour que le BC `teams` puisse le consommer.

---

## Conception

Le pattern est identique au `competitions_app_event_publisher` existant.

### App event

```rust
// src/app/team_creation/app_event.rs
pub enum TeamCreationAppEventKind {
    TeamCreated,
}

pub enum TeamCreationAppEvent {
    TeamCreated {
        event_id:     String,
        team_id:      String,
        space_id:     String,
        team_name:    String,
        roster_id:    String,
        roster_name:  String,
        coach_id:     String,
        coach_name:   String,
        treasury:     u32,    // budget restant après construction du roster
    },
}
```

### Publisher

```rust
// src/app/team_creation/io/app_events/app_event_publisher.rs
pub fn team_creation_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    // écoute event_bus, filtre TeamSubmitted, publie TeamCreated sur app_event_bus
}
```

### Rattachement dans `TeamCreationContext`

```rust
// context.rs
pub fn init_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    team_creation_app_event_publisher(event_bus, app_event_bus);
}
```

Appel dans `main.rs` après instanciation des deux bus.

---

## Checklist

- [ ] `TeamCreationAppEvent` + `TeamCreationAppEventKind` dans `app_event.rs`
- [ ] `team_creation_app_event_publisher()` : listen → filter → translate → publish
- [ ] `init_app_event_publisher()` dans `TeamCreationContext`
- [ ] Appel dans `main.rs`
- [ ] Vérifier que le `TeamSubmittedEvent` contient bien `coach_name` et `treasury` restant (sinon enrichir)
