# BC match_report — App event MatchReportConfirmed + listener teams

**Priorité : haute**
**Dépend de :** 93, 87
**Contexte :** match_report step1, verrouillage des équipes

## Objectif

Émettre un app event `MatchReportConfirmed` depuis le BC match_report quand la sélection est confirmée, et l'écouter dans le BC teams pour passer les deux équipes en `MatchReporting`.

## Conception

Cf. `docs/specs/match-report/step1-selection/07-integration.md`

### App event (shared_kernel)

Fichier : `src/app/shared_kernel/app_events/match_report_app_events.rs` (nouveau)

```rust
pub enum MatchReportAppEvent {
    MatchReportConfirmed {
        event_id: String,
        match_report_id: String,
        home_team_id: String,
        away_team_id: String,
        space_id: String,
    },
}
```

### Émission (BC match_report)

Émettre depuis le `UpdateMatchSelectionUseCase` après l'append du `SelectionConfirmed`.

Nécessite un app event publisher dans le BC match_report :
- `src/app/match_report/io/app_events/app_event_publisher.rs`

### Listener (BC teams)

Fichier : `src/app/teams/io/app_events/match_report_confirmed_listener.rs` (nouveau)

Pour chaque équipe (home + away) :
1. Charger l'agrégat Team depuis l'event store
2. Appeler `team.start_match_reporting()` → produit `MatchReportingStarted` domain event
3. Appender l'event

La méthode `start_match_reporting()` vérifie que la team est en `ReadyToPlay` → sinon erreur (cas défensif, normalement vérifié en amont par le use case).

Initialisation dans `TeamsContext::init_listeners()`.

## Checklist

- [ ] Créer `match_report_app_events.rs` dans shared_kernel
- [ ] Créer le publisher dans le BC match_report
- [ ] Émettre `MatchReportConfirmed` depuis `UpdateMatchSelectionUseCase`
- [ ] Méthode domaine `Team::start_match_reporting()` → `MatchReportingStarted` event
- [ ] `match_report_confirmed_listener.rs` dans le BC teams
- [ ] Initialisation du listener dans `TeamsContext`
- [ ] `cargo check` passe
- [ ] Test unitaire : `start_match_reporting` sur team ReadyToPlay → OK
- [ ] Test unitaire : `start_match_reporting` sur team en autre phase → erreur
- [ ] Test d'intégration : confirmer un rapport → les deux teams passent en MatchReporting
