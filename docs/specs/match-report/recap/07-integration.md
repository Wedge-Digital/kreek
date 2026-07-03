# Récap — Phase 7 : Intégration ✅

## Migration SQL

**Aucune migration nécessaire.** `match_report_proj` a déjà une colonne `phase` (TEXT) qui reflète l'état — mirroring le pattern `PostMatchRecorded → phase = 'ReadyToPublish'` déjà en place :

```rust
// src/app/match_report/io/repository/match_report_repository.rs — update_projection_in_tx
MatchReportDomainEvent::MatchReportPublished { .. } => {
    sqlx::query(
        "UPDATE match_report_proj
         SET phase = 'Published', version = $2, updated_at = now()
         WHERE match_report_id = $1",
    )
    .bind(match_report_id)
    .bind(version as i64)
    .execute(&mut **tx)
    .await
    .map_err(RepositoryError::Database)?;
}
```

`find_by_id` n'a besoin d'aucun changement : la réhydratation passe par `rehydrate()` (phase 6), qui gère déjà le nouveau variant d'event.

---

## Ports — implémentations concrètes

### `ICompetitionDataPort::find_round_context` — `src/infrastructure/match_report/competition_data_adapter.rs`

```rust
async fn find_round_context(&self, season_id: &str, round_id: &str) -> Option<RoundContextDto> {
    let season = self.season_repo.find_by_id(season_id).await.ok()??;
    let round = self.season_repo.find_round_by_id(round_id).await.ok()??; // méthode à vérifier/ajouter côté ISeasonRepository si absente
    Some(RoundContextDto {
        competition_name: season.competition_name.clone(),
        season_name: season.name.clone(),
        round_name: round.name,
    })
}
```

Dégradation gracieuse : `None` en cas d'échec à n'importe quelle étape — le template masque simplement le bandeau contexte (cf. 02-front.md).

### `ICoachDataPort` — nouveau fichier `src/infrastructure/match_report/coach_data_adapter.rs`

```rust
pub struct CoachDataAdapter {
    user_cache_repo: Arc<dyn ISpaceUserCacheRepository>,
}

#[async_trait]
impl ICoachDataPort for CoachDataAdapter {
    async fn find_coach_name(&self, coach_id: &str) -> Option<String> {
        let id = CoachId::try_new(coach_id).ok()?;
        let user = self.user_cache_repo.find_user_by_id(&id).await.ok()?;
        Some(user.coach_name.to_string())
    }
}
```

### `ISppCalculatorPort` — nouveau fichier `src/infrastructure/match_report/spp_calculator_adapter.rs` (stub)

```rust
pub struct SppCalculatorAdapter;

#[async_trait]
impl ISppCalculatorPort for SppCalculatorAdapter {
    async fn calculate_match_spp(
        &self,
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
        _home_roster_id: &str,
        _away_roster_id: &str,
    ) -> SppMatchResult {
        // STUB (cf. 06-domaine.md, BR5 descopé) — appelle spp_calculator::domain::calculator::calculate(),
        // qui crédite une valeur plausible (10 SPP) à chaque acteur distinct, en excluant les
        // actions Blesse{injury} (BR5). Pas de dépendance IRosterSppPort tant que le calcul réel
        // n'est pas implémenté (carte dédiée) — le résultat ne varie pas selon le roster.
        let result = spp_calculator::domain::calculator::calculate(
            &to_spp_inputs(home_actions),
            &to_spp_inputs(away_actions),
        );
        SppMatchResult {
            home: to_player_spp_dtos(result.home, home_actions),
            away: to_player_spp_dtos(result.away, away_actions),
        }
    }
}
```

**Décision de descope confirmée pour cette carte** : `IRosterSppPort`, `roster_spp_adapter.rs`, `assets/spp_calculator/spp_rules.json` et `domain/spp_rules.rs` (phase 3/4) **ne sont pas créés maintenant** — ils n'ont aucun appelant tant que `calculate()` est un stub qui ne consulte aucun ruleset. Les créer maintenant produirait du code mort. Ils seront ajoutés avec la carte dédiée « calcul SPP réel ».

`src/app/spp_calculator/` pour cette carte se limite donc à :
```
src/app/spp_calculator/
├── mod.rs
└── domain/
    └── calculator.rs   ← calculate() stub (06-domaine.md)
```

---

## Événements — bus interne + publisher (décision Phase 5, validée)

### `MatchReportContext` — `src/app/match_report/context.rs`

```rust
pub struct MatchReportContext {
    // ... champs existants ...
    pub event_bus: EventBus,   // NOUVEAU — bus interne au BC
}

pub fn init_listeners(app_event_bus: &EventBus, event_bus: &EventBus, pool: PgPool) {
    let repo = /* ... */;
    pairing_created_listener::init(app_event_bus, repo.clone());
    pairing_deleted_listener::init(app_event_bus, repo);
    match_report_app_event_publisher(event_bus, app_event_bus.clone());  // NOUVEAU
}
```

### `src/app/match_report/io/app_events/app_event_publisher.rs` (nouveau)

Même pattern que `competitions_app_event_publisher` :

```rust
pub fn match_report_app_event_publisher(event_bus: &EventBus, app_event_bus: EventBus) {
    let mut rx = event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) = serde_json::from_value::<MatchReportDomainEvent>(envelope.payload.clone())
                    else { continue; };
                    if let Some(app_event) = event.to_app_event() {
                        let _ = app_event_bus.send(app_event.to_enveloppe());
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("match_report_app_event_publisher: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
```

### `MatchReportDomainEvent::to_app_event()` — `src/app/match_report/domain/events.rs`

```rust
impl MatchReportDomainEvent {
    pub fn to_app_event(&self) -> Option<MatchReportAppEvent> {
        match self {
            Self::MatchReportPublished { published_by, published_at } => {
                // Le use case fournit le MatchReportPublished complet séparément
                // (le domain event seul ne porte pas toutes les données du payload) —
                // voir note ci-dessous sur la construction du payload.
                None // placeholder — cf. note
            }
            _ => None,
        }
    }
}
```

**Point d'attention pour l'implémentation** : le payload `MatchReportPublishedPayload` (validé phase 2, cf. HANDOFF.md) est riche — il inclut les actions et temp players des deux équipes, pas seulement `published_by`/`published_at`. Le domain event `MatchReportPublished` (phase 6) ne porte que le delta minimal (cohérent avec les autres events de ce BC, ex. `FanFactorRecorded` ne porte que le delta, pas tout l'état). Le mapping domain event → app event a donc besoin de **l'état complet** (`MatchReportPublished`), pas seulement de l'event. Solution retenue : `publish_match_report_use_case::execute` construit et transmet le state complet au publisher via une variante enrichie de l'event, ou le publisher recharge l'état via `repo.find_by_id` avant de mapper. **Ce point technique est à trancher précisément en phase 8 (carte dédiée « publisher + AppEvent »)** — les deux options sont réalisables, aucune ne change le contrat externe (`MatchReportAppEvent::MatchReportPublished`).

### `MatchReportAppEvent::MatchReportPublished` — `src/app/shared_kernel/app_events/match_report_app_events.rs`

Reprend intégralement la structure validée en phase 2 (HANDOFF.md) :

```rust
MatchReportPublished(MatchReportPublishedPayload),
```

```rust
pub struct MatchReportPublishedPayload {
    pub match_report_id: String,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub round_id: String,
    pub pairing_id: Option<String>,
    pub published_at: DateTime<Utc>,
    pub home_team_id: String,
    pub away_team_id: String,
    pub home_score: u8,
    pub away_score: u8,
    pub home_gain_kpo: u32,
    pub away_gain_kpo: u32,
    pub home_fan_mod: i8,
    pub away_fan_mod: i8,
    pub home_actions: Vec<MatchActionPublishedPayload>,
    pub away_actions: Vec<MatchActionPublishedPayload>,
    pub home_temp_players: Vec<TempPlayerPayload>,
    pub away_temp_players: Vec<TempPlayerPayload>,
}

pub struct MatchActionPublishedPayload {
    pub turn: u8,
    pub player: PlayerRefPayload,
    pub action: ActionTypePayload,
}

pub enum PlayerRefPayload {
    Regular { player_id: String },
    Star { ref_uid: String, display_name: String },
    Mercenary,
    Journalier,
}

pub struct TempPlayerPayload {
    pub id: String,
    pub kind: String,          // "StarPlayer" | "Mercenary" | "Journalier"
    pub display_name: Option<String>,
}
```

Consommateurs (existants, hors scope de cette page — juste le contrat) : BC `teams`, BC `players`.

---

## Routes — `src/app/match_report/routes.rs`

```rust
pub const MATCH_REPORT_RECAP: &str =
    "/app/{space_id}/match-report/{match_report_id}/recap";
pub const MATCH_REPORT_RECAP_PUBLISH: &str =
    "/app/{space_id}/match-report/{match_report_id}/recap/publish";
```

```rust
pub fn recap(&self, space_id: &str, match_report_id: &str) -> String {
    path::MATCH_REPORT_RECAP
        .replace("{space_id}", space_id)
        .replace("{match_report_id}", match_report_id)
}

pub fn recap_publish(&self, space_id: &str, match_report_id: &str) -> String {
    path::MATCH_REPORT_RECAP_PUBLISH
        .replace("{space_id}", space_id)
        .replace("{match_report_id}", match_report_id)
}
```

## Router — `src/app/match_report/router.rs`

```rust
.route(path::MATCH_REPORT_RECAP, get(get_recap))
.route(path::MATCH_REPORT_RECAP_PUBLISH, post(post_publish))
```

Montées dans le router `protected` (comme le reste de `match_report`) — `require_auth` suffit (BR8, pas de permission plus fine).

---

## Handler — `src/app/match_report/io/web/recap_controller.rs` (nouveau)

```rust
pub async fn get_recap(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if auth_session.user.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match build_recap_vm(&space_id, &match_report_id, &state).await {
        Ok(tpl) => tpl.into_response(),
        Err(status) => status.into_response(),
    }
}
```

`build_recap_vm` (fonction privée, < 20 lignes) :
1. Charge l'état via `repo.find_by_id` → 404 si absent
2. Match sur l'état : `Draft`/`PreMatch` → 404 ; `Cancelled` → 410 Gone ; `ReadyToPublish`/`Published` → poursuite
3. Appelle `builders::build_team_banner` (×2), `builders::build_round_context_vm`, `builders::build_performance_rows`, `builders::build_submitted_by`
4. Appelle les constructeurs domaine (`view_models.rs`) pour `MatchResultVm`, `GainsFanVm`, `TimelineEventVm::all_from_domain`, `MvpRowVm::all_from_domain`, `InjuryRowVm::all_from_domain`
5. Construit les URLs (`publish_url`, `back_to_step5_url`, `competition_url` via `AppRoutes::default().competitions.competition_detail(...)`, `home_team_detail_url` via `AppRoutes::default().teams.team_detail(...)`)
6. Retourne `RecapTemplate`

```rust
pub async fn post_publish(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else { return StatusCode::UNAUTHORIZED.into_response(); };
    let Ok(mr_id) = MatchReportId::try_new(&match_report_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = PublishMatchReportCommand { match_report_id: mr_id, published_by: user.id };
    match publish_match_report_use_case::execute(cmd, state.match_report.repo.as_ref(), &state.match_report.event_bus).await {
        Ok(()) => Redirect::to(&AppRoutes::default().match_report.recap(&space_id, &match_report_id)).into_response(),
        Err(PublishMatchReportError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(PublishMatchReportError::AlreadyPublished) => StatusCode::CONFLICT.into_response(),
        Err(PublishMatchReportError::Cancelled) => StatusCode::GONE.into_response(),
        Err(PublishMatchReportError::Repository(e)) => {
            tracing::error!("post_publish: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
```

Formulaire HTML : `<form method="post">` classique (pas de `hx-post`), cohérent avec `pre-match.html`/`step5.html`/`inducements.html` — navigation complète, pas de fragment HTMX (02-front.md : « Pas de HTMX partiel »).

---

## Template — `src/app/match_report/io/web/templates/recap.html` (nouveau)

Structure miroir de la maquette (`assets/rawpages/html/app-match-summary.html`), avec les ajustements déjà actés :
- Hero : bandeau contexte (`round_context`, masqué si `None`) + scoreboard (`home_banner`/`away_banner` + score) + stats strip (TDs, **Blessures uniquement — pas de KO**, fan factors)
- Colonne principale : compte-rendu (`summary_title`/`summary_body`, byline `submitted_by` si `Some`), chronologie (`timeline`, toutes les actions + `mvps` en fin de liste), CTA de bas de page (dépend de `is_published`)
- Sidebar : « Joueurs du match » (`mvps`, même donnée que la chronologie), « Performances (SPP) » (`performances` — vide tant que le stub est en place), « Gains & Fan Factor » (`gains_fan`, delta uniquement), « Bilan sanitaire » (`injuries`, filtré `Blesse{injury}` uniquement)
- CTA bas de page : deux variantes selon `is_published` (cf. 02-front.md) :
  - `false` → `<a href="{{ back_to_step5_url }}">← Modifier étape 5</a>` + `<form method="post" action="{{ publish_url }}"><button type="submit">Publier</button></form>`
  - `true` → `<a href="{{ competition_url }}">← Retour compétition</a>` + `<a href="{{ home_team_detail_url }}">Voir fiche {{ home_banner.team_name }} →</a>`

CSS : `assets/static/css/pages/match-report-recap.css` (nouveau, réutilise les classes `.mr-*` partagées si pertinent, sinon namespace `.ms-*` propre à cette page).

---

## Fichiers créés / modifiés — récapitulatif

| Action | Fichier |
|---|---|
| CRÉÉ | `src/app/match_report/io/web/recap_controller.rs` |
| CRÉÉ | `src/app/match_report/io/web/templates/recap.html` |
| CRÉÉ | `src/app/match_report/io/web/builders.rs` (ou extension si déjà existant) |
| CRÉÉ | `src/app/match_report/io/web/view_models.rs` (ou extension) |
| CRÉÉ | `src/app/match_report/use_cases/publish_match_report_use_case.rs` |
| CRÉÉ | `src/app/match_report/domain/match_report_published.rs` |
| CRÉÉ | `src/app/match_report/io/app_events/app_event_publisher.rs` |
| CRÉÉ | `src/infrastructure/match_report/coach_data_adapter.rs` |
| CRÉÉ | `src/infrastructure/match_report/spp_calculator_adapter.rs` (stub) |
| CRÉÉ | `src/app/spp_calculator/mod.rs` + `domain/calculator.rs` (stub) |
| CRÉÉ | `assets/static/css/pages/match-report-recap.css` |
| MODIFIÉ | `src/app/match_report/domain/match_report_ready_to_publish.rs` — `publish()` |
| MODIFIÉ | `src/app/match_report/domain/events.rs` — variant + `to_app_event()` |
| MODIFIÉ | `src/app/match_report/domain/match_report_state.rs` — variant `Published` + rehydrate |
| MODIFIÉ | `src/app/match_report/ports.rs` — `find_round_context`, `ISppCalculatorPort`, `ICoachDataPort` |
| MODIFIÉ | `src/infrastructure/match_report/competition_data_adapter.rs` — `find_round_context` |
| MODIFIÉ | `src/app/shared_kernel/app_events/match_report_app_events.rs` — variant `MatchReportPublished` |
| MODIFIÉ | `src/app/match_report/context.rs` — `event_bus` interne + wiring publisher |
| MODIFIÉ | `src/app/match_report/routes.rs` / `router.rs` |
| MODIFIÉ | `main.rs` — instanciation des nouveaux adapters + bus interne |

---

## Plan de tests E2E (Playwright / pytest)

Fichier cible : `tests/e2e/test_match_report_recap.py` (nouveau)

### Prérequis

Match report en état `ReadyToPublish` (fan factor + inducements + actions + post-match déjà soumis, cf. fixtures des fichiers `test_match_report_step5.py`).

### TC-RECAP-01 — Page recap charge en état ReadyToPublish

```
1. Naviguer vers /recap d'un match ReadyToPublish
2. Vérifier bandeau équipes, score, gains/fan factor visibles
3. Vérifier CTA "Publier" + "← Modifier étape 5" visibles (pas "Retour compétition")
```

### TC-RECAP-02 — Chronologie et bilan sanitaire cohérents

```
1. Charger la page recap d'un match avec au moins une action Blesse et une action Sortie
2. Vérifier que la Sortie apparaît dans la chronologie SANS badge de blessure
3. Vérifier que seule l'action Blesse apparaît dans la carte "Bilan sanitaire"
```

### TC-RECAP-03 — MVP affiché aux deux endroits

```
1. Charger une page recap avec un MVP désigné de chaque côté
2. Vérifier la présence dans la carte sidebar "Joueurs du match"
3. Vérifier la présence en fin de chronologie
```

### TC-RECAP-04 — Publication → état Published

```
1. Cliquer "Publier"
2. Vérifier la redirection vers la même page /recap
3. Vérifier que le CTA a changé : "← Retour compétition" + "Voir fiche {équipe}"
```

### TC-RECAP-05 — Double publication refusée

```
1. Sur un match déjà Published, POST /recap/publish directement (requests, pas navigateur)
2. Vérifier status 409
```

### TC-RECAP-06 — États non accessibles

```
1. GET /recap sur un match Draft ou PreMatch → 404
2. GET /recap sur un match Cancelled → 410
```

### TC-RECAP-07 — Dégradation gracieuse round_context

```
1. Charger un recap dont find_round_context échoue (données incomplètes)
2. Vérifier que la page se charge quand même, sans le bandeau contexte compétition/saison/journée
```

### TC-RECAP-08 — Performances SPP vides (stub)

```
1. Charger la page recap
2. Vérifier que la carte "Performances (SPP)" ne casse pas l'affichage même vide (stub retourne toujours [])
```
