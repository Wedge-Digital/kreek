# 130 — Enrichissement des events PairingCreated et MatchReportConfirmed

## Objectif

Enrichir deux events existants avec les données d'affichage nécessaires à la projection `competition_match_display_proj` :
- `PairingCreated` (domain event competitions) : ajouter les données d'équipes (noms, logos, roster, coach, dates de journée)
- `MatchReportConfirmed` (app event match_report) : ajouter `pairing_id`

## Dépendances

Aucune — première carte de la feature.

## Conception détaillée

### 1. `domain/domain_event.rs` — `PairingCreated`

Ajouter les champs suivants à la variante existante :

```rust
PairingCreated {
    // champs existants conservés à l'identique
    event_id: EventId,
    pairing_id: String,
    competition_id: String,
    season_id: String,
    round_id: String,
    home_team_id: String,
    away_team_id: String,
    space_id: String,
    // nouveaux champs
    home_team_name: String,
    home_roster_name: String,
    home_coach_name: String,
    home_logo_url: Option<String>,
    away_team_name: String,
    away_roster_name: String,
    away_coach_name: String,
    away_logo_url: Option<String>,
    round_name: String,
    round_position: i32,
    round_date_start: Option<String>,
    round_date_end: Option<String>,
    round_day_type: String,  // "fixed_date" | "time_frame" | "rest"
}
```

### 2. Use cases `generate_pairings.rs` et `generate_all_pairings.rs`

Ces use cases créent les `PairingCreated` events. Ils doivent charger les données d'affichage des équipes et de la journée pour les embarquer dans l'event.

Vérifier comment les teams sont chargées dans ces use cases et passer les champs display au constructeur du domaine event.

### 3. `shared_kernel/app_events/match_report_app_events.rs` — `MatchReportConfirmed`

Ajouter `pairing_id: Option<String>` :

```rust
MatchReportConfirmed {
    event_id: EventId,
    match_report_id: String,
    home_team_id: String,
    away_team_id: String,
    space_id: String,
    pairing_id: Option<String>,  // nouveau — None si rapport hors compétition
}
```

### 4. `use_cases/create_match_report_use_case.rs` et `update_match_selection_use_case.rs`

Ces deux use cases émettent `MatchReportConfirmed`. Passer `pairing_id` depuis l'agrégat `MatchReport`.

## Checklist

- [ ] Variante `PairingCreated` enrichie dans `domain_event.rs`
- [ ] `generate_pairings.rs` : emit `PairingCreated` avec les champs display
- [ ] `generate_all_pairings.rs` : idem
- [ ] `MatchReportConfirmed` app event enrichi avec `pairing_id`
- [ ] `create_match_report_use_case.rs` : passer `pairing_id`
- [ ] `update_match_selection_use_case.rs` : passer `pairing_id`
- [ ] `cargo build` passe sans erreur
