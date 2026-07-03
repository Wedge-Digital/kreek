# MR-RECAP-04 — Bus interne + publisher + AppEvent

## Objectif

Mettre le BC `match_report` en conformité avec la règle CLAUDE.md « Émission des app events » :
ajouter un bus interne au BC, créer son `app_event_publisher.rs`, et définir l'AppEvent
`MatchReportPublished` consommé par BC Teams et BC Players.

## Dépendances

144 — `MatchReportDomainEvent::MatchReportPublished` et `MatchReportPublished` doivent exister
pour construire le mapping `to_app_event()`.

## Conception

Voir `docs/specs/match-report/recap/05-use-cases.md` (section « Décision — émission de
l'AppEvent ») et `docs/specs/match-report/recap/07-integration.md`.

**Ne pas toucher** `create_match_report_use_case.rs` — l'écart legacy existant (émission directe
via `app_event_bus` passé au use case) n'est pas corrigé dans cette carte, seul le nouveau code
suit la règle actuelle.

## Fichiers impactés

- `src/app/match_report/context.rs`
- `src/app/match_report/io/app_events/app_event_publisher.rs` (nouveau)
- `src/app/match_report/domain/events.rs`
- `src/app/shared_kernel/app_events/match_report_app_events.rs`
- `main.rs`

## Checklist

### Bus interne
- [ ] Champ `event_bus: EventBus` ajouté à `MatchReportContext`
- [ ] Instanciation dans `main.rs`, injectée dans le contexte

### Publisher
- [ ] `match_report_app_event_publisher(event_bus, app_event_bus)` — même pattern que `competitions_app_event_publisher`
- [ ] Appelé dans `MatchReportContext::init_listeners` (ou équivalent)

### AppEvent
- [ ] `MatchReportAppEvent::MatchReportPublished(MatchReportPublishedPayload)` dans `match_report_app_events.rs`
- [ ] `MatchReportPublishedPayload` — structure complète validée en phase 2 (cf. HANDOFF.md) : identifiants, scores, gains, fan mods, actions (`MatchActionPublishedPayload`/`PlayerRefPayload`), temp players (`TempPlayerPayload`)
- [ ] `MatchReportDomainEvent::to_app_event()` — mapping `MatchReportPublished` (domain event) → `MatchReportAppEvent::MatchReportPublished` (app event)
- [ ] **Trancher et implémenter** le point technique laissé ouvert en phase 7 : le domain event ne porte que `published_by`/`published_at`, il faut recharger l'état complet (`repo.find_by_id`) côté publisher, ou enrichir l'event — choisir l'option la plus simple à l'implémentation et documenter le choix dans le code

### Build
- [ ] Compiler sans erreur (`cargo build`)
- [ ] Test d'intégration : publier un match report → vérifier qu'un `MatchReportAppEvent::MatchReportPublished` est bien émis sur l'app event bus
