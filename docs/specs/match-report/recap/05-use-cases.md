# Récap — Phase 5 : Use cases ✅

## Contexte

Une seule mutation sur cette page : `POST /recap/publish`. Le `GET /recap` reste une composition de lecture pure (controller + `builders.rs`/`view_models.rs`, cf. phases 3-4), pas de use case dédié.

## Use case : `publish_match_report_use_case.rs` — BC `match_report`

Fichier : `src/app/match_report/use_cases/publish_match_report_use_case.rs`

### Signature

```rust
pub struct PublishMatchReportCommand {
    pub match_report_id: MatchReportId,
    pub published_by: CoachId,
}

pub async fn execute(
    cmd: PublishMatchReportCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,   // bus interne au BC match_report — cf. décision ci-dessous
) -> Result<(), PublishMatchReportError>
```

### Orchestration

1. Charger l'état via `repo.find_by_id(&cmd.match_report_id.to_string())`
2. Vérifier l'état courant (règle d'accès validée en 02-front.md) :
   - `Draft` / `PreMatch` → `PublishMatchReportError::NotFound` (404, cohérent avec le fait que la page recap n'existe pas encore à ces stades)
   - `ReadyToPublish` → suite du flux
   - un futur variant `Published` (ajouté en phase 6) → `PublishMatchReportError::AlreadyPublished` (409)
   - `Cancelled` → `PublishMatchReportError::Cancelled` (410)
3. Appeler la méthode domaine `rtp.publish(cmd.published_by)` (phase 6) → `(MatchReportPublished, MatchReportDomainEvent::MatchReportPublished)`
4. Persister l'événement (`repo.append`, version courante)
5. Émettre l'événement domaine sur le bus interne du BC (`bus.send(event.to_enveloppe())`) — le publisher du BC (phase 7) le convertit en `MatchReportAppEvent::MatchReportPublished` et le republie sur l'app event bus

### Erreurs

```rust
#[derive(Debug)]
pub enum PublishMatchReportError {
    NotFound,
    AlreadyPublished,
    Cancelled,
    Repository(String),
}
```

Mapping HTTP (côté `recap_controller.rs`, phase 7) : `NotFound` → 404, `AlreadyPublished` → 409, `Cancelled` → 410, `Repository` → 500.

## Décision — émission de l'AppEvent (validée)

Vérification du code existant : `create_match_report_use_case.rs` (`confirm_existing`, L69-105) reçoit **directement** `app_event_bus: &EventBus` en paramètre et appelle `app_event_bus.send(...)` depuis le use case — ce qui contredit la règle CLAUDE.md « Émission des app events » (l'`app_event_bus` ne doit jamais être passé à un use case). Le BC `match_report` n'a aujourd'hui ni bus interne, ni `io/app_events/app_event_publisher.rs` (vérifié : absent, contrairement à `competitions`, `team_creation`, `auth` qui en ont un).

**Validé** : mettre le BC `match_report` en conformité pour ce nouveau use case plutôt que reconduire l'écart — le CLAUDE.md est explicite : « tout nouveau fichier... doit suivre [les conventions] ». Concrètement (détail complet en phase 7) :
- Ajouter un `event_bus: EventBus` interne au `MatchReportContext` (mirroring `CompetitionsContext`)
- Créer `src/app/match_report/io/app_events/app_event_publisher.rs` (même pattern que `competitions_app_event_publisher`)
- `publish_match_report_use_case::execute` prend ce bus interne, pas l'`app_event_bus`
- Le fichier existant `create_match_report_use_case.rs` **n'est pas touché** (pas de refonte massive, cf. CLAUDE.md) — l'écart legacy y reste, seul le nouveau code suit la règle actuelle
