# BC `teams` — Consommation `TeamCreated` → équipe "En attente d'inscription"

**Priorité : haute**
**Dépend de :** `29-teams-repository.md`, `30-team-created-app-event.md`
**Contexte :** `teams` (consommateur)

## Objectif

Créer dans le BC `teams` un listener sur l'app event bus qui réagit à `TeamCreated` en instanciant un agrégat `Team` en statut `PendingEnrollment` et le persiste.

---

## Conception

### Listener

L'app event `TeamCreated` est traduit en `TeamDomainEvent::TeamCreated` et appendé dans l'event store. C'est le premier événement du flux de l'équipe — version attendue = 0.

```rust
// io/app_events/team_created_listener.rs
pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) = serde_json::from_value::<TeamCreationAppEvent>(envelope.payload.clone()) else {
                        continue;
                    };
                    if let TeamCreationAppEvent::TeamCreated { team_id, space_id, name,
                                                               roster_id, roster_name,
                                                               coach_id, coach_name, treasury } = app_event
                    {
                        let domain_event = TeamDomainEvent::TeamCreated {
                            team_id, space_id, name, roster_id, roster_name,
                            coach_id, coach_name, treasury,
                        };
                        // version 0 → premier événement de ce team_id
                        if let Err(e) = team_repo.append(&TeamId::from(&domain_event.team_id()), &domain_event, 0).await {
                            tracing::error!("teams: failed to append TeamCreated: {e}");
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("teams team_created_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
```

### Idempotence

Si le listener reçoit deux fois le même `event_id`, le second `append` échoue avec `RepositoryError::ConcurrentWrite` (contrainte unique sur `(team_id, version)`). Le listener logue l'erreur et continue — l'événement est déjà en base.

### Rattachement dans `TeamsContext`

```rust
pub fn init_listeners(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    team_created_listener::init(app_event_bus, team_repo);
}
```

---

## Checklist

- [ ] `team_created_listener::init()` — subscribe, filter, construire `TeamDomainEvent::TeamCreated`, `team_repo.append(..., 0)`
- [ ] Gestion idempotente de `ConcurrentWrite` (log + continue)
- [ ] `TeamsContext::init_listeners()` appelé depuis `main.rs`
- [ ] Test d'intégration : publier un app event `TeamCreated` → vérifier l'événement en base + projection
