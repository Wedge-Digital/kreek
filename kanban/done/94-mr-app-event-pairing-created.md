# BC match_report — App event PairingCreated

**Priorité : haute**
**Dépend de :** 93
**Contexte :** match_report step1, communication inter-BCs

## Objectif

Émettre un app event `PairingCreated` depuis le BC competitions quand un pairing est créé, et l'écouter dans le BC match_report pour créer automatiquement un rapport en phase Draft.

## Conception

Cf. `docs/specs/match-report/step1-selection/07-integration.md`

### Émission (BC competitions)

Fichier : `src/app/shared_kernel/app_events/competitions_app_events.rs`

Ajouter le variant :

```rust
PairingCreated {
    event_id: String,
    pairing_id: String,
    season_id: String,
    round_id: String,
    home_team_id: String,
    away_team_id: String,
    space_id: String,
}
```

Émettre depuis les use cases :
- `generate_pairings` (génération automatique)
- `generate_all_pairings` (génération toutes journées)
- handler `add_match` (ajout manuel dans le calendrier admin)

### Listener (BC match_report)

Fichier : `src/app/match_report/io/app_events/pairing_created_listener.rs`

Écoute `PairingCreated` → construit un `CreateMatchReportCommand` avec `origin: Pairing` → appelle `CreateMatchReportUseCase`.

Initialisation dans `context.rs` via `init_listeners()`.

## Checklist

- [ ] Ajouter `PairingCreated` dans `competitions_app_events.rs`
- [ ] Émettre depuis `generate_pairings` use case
- [ ] Émettre depuis `generate_all_pairings` use case
- [ ] Émettre depuis le handler `add_match` (schedule admin)
- [ ] `pairing_created_listener.rs` : écoute + appelle CreateUseCase
- [ ] Initialisation du listener dans `MatchReportContext::init_listeners()`
- [ ] `cargo check` passe
- [ ] Test : générer des pairings → vérifier qu'un match report Draft existe pour chaque pairing
