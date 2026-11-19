# MR-RECAP-09 — Listener MatchReportPublished côté competitions

## Objectif

Câbler dans le BC `competitions` un listener qui consomme l'app event
`MatchReportAppEvent::MatchReportPublished` (émis depuis la carte 148) pour mettre à jour
la projection `competition_match_display_proj` — celle qui alimente l'onglet Résultats.

Sans ce listener, un match publié reste affiché comme "en cours" avec un lien mort vers
l'ancienne URL d'édition (`edit_match_report` renvoie 409 pour l'état `Published`) — bug
constaté en vérifiant le rework front de la page récap (carte 149).

## Dépendances

147/148 — le publisher et l'event `MatchReportPublished` sont déjà émis.

## Conception

Miroir du listener frère déjà en place pour `MatchReportConfirmed`
(`src/app/competitions/io/app_events/match_report_confirmed_listener.rs`) — même
souscription au bus, même garde `pairing_id: Some(...)` (skip si match manuel sans pairing).

```sql
UPDATE competition_match_display_proj
SET match_status = 'completed',
    home_score = $2,
    away_score = $3,
    home_casualties = $4,
    away_casualties = $5,
    match_report_url = $6
WHERE pairing_id = $1
```

- `home_score`/`away_score` : directement dans `MatchReportPublishedPayload`
- `home_casualties`/`away_casualties` : dérivés du payload — compte des actions
  `ActionTypePayload::Sortie` par équipe (même définition que
  `MatchReportPreMatch::compute_cas()` côté `match_report`). Extrait dans une fonction pure
  `count_casualties(actions: &[MatchActionPublishedPayload]) -> u8`, testable sans DB.
- `match_report_url` : reconstruit via `AppRoutes::default().match_report.recap(&space_id, &match_report_id)`
  (au lieu de `edit_match_report`)

`resultats_tab_controller.rs` / `list_resultats.sql` / `PairingDisplayDto` ne changent pas —
`is_completed` est déjà dérivé de `match_status == "completed"`.

## Fichiers impactés

- `src/app/competitions/io/app_events/match_report_published_listener.rs` (nouveau)
- `src/app/competitions/io/app_events/mod.rs`
- `src/app/competitions/context.rs`

## Checklist

- [ ] `count_casualties(actions: &[MatchActionPublishedPayload]) -> u8` — filtre `ActionTypePayload::Sortie` uniquement (pas `Blesse`)
- [ ] `init(app_event_bus, pool)` — souscrit au bus, pattern-match `MatchReportAppEvent::MatchReportPublished`, skip si `pairing_id: None`
- [ ] `handle_event` — construit l'URL recap via `AppRoutes`, exécute l'UPDATE SQL, log en erreur si échec (pas de panic)
- [ ] Déclaration module + wiring dans `context.rs`
- [ ] Tests unitaires `count_casualties` (vide, sorties uniquement, ignore Blesse)
- [ ] Test E2E : faire aller un match jusqu'à `Published`, vérifier via DB que `match_status='completed'` et `match_report_url` contient `/recap`, puis vérifier à l'écran que la page Résultats affiche le score et un lien qui charge (pas de 409)
- [ ] Compiler sans erreur (`cargo build`)
- [ ] Tests verts (`make test`)
