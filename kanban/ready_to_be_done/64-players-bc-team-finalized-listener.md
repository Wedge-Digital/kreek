# BC `players` — Listener `TeamCreated` → création des agrégats joueurs

**Priorité : haute**
**Dépend de :** `63-players-bc-projection.md`
**Contexte :** BC `players` — couche IO (listener app event)

## Objectif

Écouter l'app event `TeamCreated` publié par BC `team_creation`, et créer une instance
`PlayerCreated` pour chaque joueur recruté dans l'équipe finalisée.

---

## ⚠ Prérequis : enrichissement de `TeamCreated`

Le `TeamCreated` app event actuel ne contient **pas les joueurs**. Il faut l'enrichir
avec la liste des joueurs avant d'implémenter ce listener.

### Enrichissement de `TeamCreationAppEvent::TeamCreated`

```rust
// shared_kernel/app_events/team_creation_app_events.rs
pub struct PlayerPayload {
    pub instance_id:     String,   // = PlayerId dans BC players
    pub roster_line_id:  String,
    pub position_name:   String,
    pub personal_name:   String,
    pub jersey:          Option<u8>,
    pub base_skills:     Vec<String>,
    pub acquired_skills: Vec<AcquiredSkillPayload>,
    pub spp_spent:       u8,       // SPP dépensés pendant la finalisation (pour starting_value)
}

pub struct AcquiredSkillPayload {
    pub skill_id: String,
    pub mode:     String,   // "Chosen" | "Random"
    pub spp_cost: u8,
}

// Ajout dans TeamCreated :
pub players: Vec<PlayerPayload>,
```

Les deux use cases qui publient l'event (`submit_team.rs` et `batch_finalize.rs`)
doivent être mis à jour pour peupler ce champ depuis `team.hired_players()`.

---

## Listener

```rust
// src/app/players/io/app_events/team_created_listener.rs

pub async fn handle_team_created(
    event:          &TeamCreationAppEvent::TeamCreated,
    player_repo:    &dyn IPlayerRepository,
    proj_repo:      &dyn IPlayerProjectionRepository,
    ref_repo:       &dyn IReferenceRepository,
    pool:           &PgPool,
) -> Result<(), ListenerError> {
    for player_payload in &event.players {
        let starting_value = compute_starting_value(player_payload, ref_repo);

        let domain_event = PlayerDomainEvent::PlayerCreated {
            player_id:       PlayerId(player_payload.instance_id.clone()),
            team_id:         TeamId(event.team_id.clone()),
            space_id:        event.space_id.clone(),
            position_name:   player_payload.position_name.clone(),
            roster_line_id:  player_payload.roster_line_id.clone(),
            personal_name:   player_payload.personal_name.clone(),
            jersey:          player_payload.jersey,
            base_skills:     player_payload.base_skills.clone(),
            acquired_skills: map_acquired_skills(&player_payload.acquired_skills),
            starting_spp:    Spp(0),
            starting_value,
        };

        let mut tx = pool.begin().await?;
        insert_player_event(&mut tx, &domain_event, 1).await?;
        upsert_player_projection(&mut tx, &domain_event).await?;
        tx.commit().await?;
    }
    Ok(())
}
```

### Calcul de `starting_value`

```
starting_value = position.cost / 1000 (kPo)
               + Σ (spp_cost × valeur en kPo selon skill_cost.json level 1)
```

Le mapping SPP → kPo est : primary chosen = 6 SPP / +10 kPo, secondary chosen = 10 SPP / +20 kPo,
random = 3 SPP / +10 kPo. Ces valeurs viennent du référentiel `skill_cost.json`.

---

## Idempotence

Si un `PlayerCreated` existe déjà pour un `player_id` donné (rediffusion de l'event),
la contrainte `UNIQUE (player_id, version)` sur `players_events` retourne une erreur
de contrainte → le listener log et ignore (`ListenerError::AlreadyProcessed`).

---

## Abonnement sur le bus

Le listener s'abonne au **bus app** (pas au bus domaine) — même pattern que
`teams/io/app_events/team_created_listener.rs` :

```rust
// src/app/players/io/app_events/team_created_listener.rs
pub fn init(app_event_bus: &EventBus, ctx: Arc<PlayersContext>, pool: PgPool, refs: Arc<dyn IReferenceRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(app_event) =
                        serde_json::from_value::<TeamCreationAppEvent>(envelope.payload.clone())
                    else { continue; };
                    let TeamCreationAppEvent::TeamCreated { team_id, space_id, players, .. } = app_event;
                    if let Err(e) = handle_team_created(&team_id, &space_id, &players, &pool, &refs).await {
                        tracing::error!("players team_created_listener: {e}");
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("players team_created_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
```

Appelé depuis `main.rs` avec `state.app_event_bus` (pas le bus domaine de `team_creation`).

### Chaîne complète déjà en place dans le code

```
batch_finalize / submit_team
  └─ domain_bus.send(TeamSubmitted)          ← use case, bus domaine
       └─ team_creation_app_event_publisher  ← IO, abonné au bus domaine
            └─ event.to_app_event() → TeamCreated
                 └─ app_event_bus.send(...)
                      └─ players/team_created_listener  ← CETTE CARTE, bus app
```

Le publisher et les deux bus existent déjà. Seuls le listener BC `players`
et l'enrichissement du payload sont à créer.

---

## Checklist

- [ ] Enrichir `PlayerPayload` + `AcquiredSkillPayload` dans `shared_kernel`
- [ ] Enrichir `TeamCreationAppEvent::TeamCreated` avec `players: Vec<PlayerPayload>`
- [ ] Mettre à jour `submit_team.rs` pour peupler `players`
- [ ] Mettre à jour `batch_finalize.rs` pour peupler `players`
- [ ] `compute_starting_value()` depuis `ref_repo`
- [ ] `handle_team_created()` avec transaction par joueur
- [ ] Idempotence sur contrainte de version
- [ ] `register_listeners()` câblé dans `main.rs`
