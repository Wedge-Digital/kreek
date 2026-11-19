# 132 — Listeners PairingCreated / PairingDeleted → projection

## Objectif

Alimenter `competition_match_display_proj` lors de la création et suppression de pairings, dans la même transaction que l'event store.

## Dépendances

- 130 (PairingCreated enrichi)
- 131 (table créée)

## Conception détaillée

Ces listeners vivent dans la couche IO du BC `competitions`. Ils écoutent le bus interne du BC (domain events), pas le bus applicatif.

### `io/app_events/pairing_projection_listener.rs` (nouveau fichier)

Deux handlers sur le bus interne :

**Sur `PairingCreated`** :
```rust
// INSERT dans competition_match_display_proj
// home_initials et away_initials calculés via initials()
// Dans la même transaction que l'append de l'event
sqlx::query!(
    "INSERT INTO competition_match_display_proj (...) VALUES (...)",
    ...
)
.execute(&mut *tx)
.await?;
```

**Sur `PairingDeleted`** :
```rust
sqlx::query!(
    "DELETE FROM competition_match_display_proj WHERE pairing_id = $1",
    pairing_id
)
.execute(&mut *tx)
.await?;
```

Les `home_initials` / `away_initials` sont calculés avec `crate::common::initials::initials()` depuis `home_team_name` / `away_team_name`.

### Enregistrement dans `context.rs`

Brancher le listener sur le bus interne du BC competitions dans `context.rs`.

## Checklist

- [ ] `pairing_projection_listener.rs` créé
- [ ] INSERT atomique avec l'event (même transaction)
- [ ] DELETE atomique avec l'event (même transaction)
- [ ] `home_initials` / `away_initials` calculés via `initials()`
- [ ] Listener enregistré dans `context.rs`
- [ ] `cargo build` passe
