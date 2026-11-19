# 133 — Listener MatchReportConfirmed → UPDATE in_progress

## Objectif

Mettre à jour `competition_match_display_proj` quand un rapport de match est démarré (selection confirmée), en passant le pairing de `upcoming` à `in_progress`.

## Dépendances

- 130 (MatchReportConfirmed enrichi avec pairing_id)
- 131 (table créée)

## Conception détaillée

### `io/app_events/match_report_confirmed_listener.rs` (nouveau fichier)

Écoute l'app event `MatchReportConfirmed` sur le bus applicatif global.

```rust
// Si pairing_id est présent dans l'event :
sqlx::query!(
    "UPDATE competition_match_display_proj
     SET match_status = 'in_progress',
         match_report_id = $2,
         match_report_url = $3
     WHERE pairing_id = $1",
    pairing_id,
    match_report_id,
    match_report_url,  // construire depuis AppRoutes
)
```

Le `match_report_url` est construit depuis `AppRoutes` (step 1 du rapport : `AppRoutes::match_report.step1(space_id, match_report_id)`).

Si `pairing_id` est `None` (rapport hors compétition), ignorer l'event.

### Enregistrement dans `context.rs`

Brancher le listener sur le bus applicatif global dans `context.rs` ou dans `main.rs`.

## Checklist

- [ ] `match_report_confirmed_listener.rs` créé dans `competitions/io/app_events/`
- [ ] UPDATE conditionnel sur `pairing_id` présent
- [ ] `match_report_url` construit via `AppRoutes`
- [ ] Listener branché sur le bus applicatif
- [ ] `cargo build` passe
