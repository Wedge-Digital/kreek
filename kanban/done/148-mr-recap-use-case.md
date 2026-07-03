# MR-RECAP-05 — Use case publish_match_report_use_case

## Objectif

Implémenter le use case qui orchestre la publication : charger l'état, vérifier qu'il est
`ReadyToPublish`, appeler `publish()`, persister, émettre l'événement sur le bus interne.

## Dépendances

144 — `MatchReportReadyToPublish::publish()` doit exister.
147 — le bus interne `event_bus` du BC doit exister pour être passé à `execute()`.

## Conception

Voir `docs/specs/match-report/recap/05-use-cases.md`.

## Fichiers impactés

- `src/app/match_report/use_cases/publish_match_report_use_case.rs` (nouveau)
- `src/app/match_report/use_cases/mod.rs`

## Checklist

- [ ] `PublishMatchReportCommand { match_report_id: MatchReportId, published_by: CoachId }`
- [ ] `PublishMatchReportError { NotFound, AlreadyPublished, Cancelled, Repository(String) }`
- [ ] `execute(cmd, repo: &dyn IMatchReportRepository, bus: &EventBus) -> Result<(), PublishMatchReportError>` :
  - [ ] Charge l'état via `repo.find_by_id`
  - [ ] `Draft`/`PreMatch` → `NotFound` ; `Published` → `AlreadyPublished` ; `Cancelled` → `Cancelled` ; `ReadyToPublish` → suite
  - [ ] Appelle `rtp.publish(cmd.published_by)`
  - [ ] Persiste l'événement (`repo.append`)
  - [ ] Émet l'événement sur `bus` (bus interne, pas l'app event bus)
- [ ] Déclaration du module dans `use_cases/mod.rs`
- [ ] Compiler sans erreur (`cargo build`)
