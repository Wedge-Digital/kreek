# MR-RECAP-01 — Domaine MatchReportPublished

## Objectif

Implémenter le nouvel état terminal `MatchReportPublished` : struct, méthode `publish()` sur
`MatchReportReadyToPublish`, nouveau domain event, nouveau variant d'état, réhydratation.

## Dépendances

Aucune — peut démarrer immédiatement.

## Conception

Voir `docs/specs/match-report/recap/06-domaine.md`.

## Fichiers impactés

- `src/app/match_report/domain/match_report_published.rs` (nouveau)
- `src/app/match_report/domain/match_report_ready_to_publish.rs`
- `src/app/match_report/domain/events.rs`
- `src/app/match_report/domain/match_report_state.rs`
- `src/app/match_report/domain/mod.rs`

## Checklist

### Nouvel agrégat `MatchReportPublished`
- [ ] Struct avec tous les champs de `MatchReportReadyToPublish` + `published_by: CoachId` + `published_at: DateTime<Utc>`
- [ ] `from_ready_to_publish(rtp, published_by, published_at) -> Self` — constructeur miroir de `MatchReportReadyToPublish::from_pre_match`
- [ ] Déclaration dans `domain/mod.rs`

### Méthode sur `MatchReportReadyToPublish`
- [ ] `publish(&self, published_by: CoachId) -> (MatchReportPublished, MatchReportDomainEvent)` — infaillible, pas de `Result`

### Événement
- [ ] `MatchReportPublished { published_by: CoachId, published_at: DateTime<Utc> }` dans `events.rs`
- [ ] `type_name()` et `schema_version()` mis à jour

### État
- [ ] Variante `Published(MatchReportPublished)` dans `MatchReportState`
- [ ] Réhydratation : `(ReadyToPublish, MatchReportPublished)` → `Published`
- [ ] Aucune branche sortante de `Published` (irréversibilité garantie par construction)

### Tests unitaires
- [ ] `publish_produces_published_state_with_all_fields_copied`
- [ ] `publish_succeeds_without_summary`
- [ ] `publish_increments_version`
- [ ] `rehydrate_ready_to_publish_then_published_yields_published_state`

### Build
- [ ] Compiler sans erreur (`cargo build`)
- [ ] Tests verts (`make test`)
