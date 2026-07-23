# BC `ranking` — Listener `MatchReportPublished` + câblage complet du BC

**Priorité : haute**
**Dépend de :** `195-ranking-use-case-record-match.md`
**Contexte :** `ranking/context.rs`, `ranking/io/app_events/`, `state.rs`, `main.rs`
**Spec :** `docs/specs/ranking/classement/07-integration.md`

## Objectif

Le BC `ranking` devient vivant : écoute `MatchReportAppEvent::MatchReportPublished` sur l'`app_event_bus`, et est enregistré dans `AppState`/`main.rs` comme les autres BCs.

## Conception

### Listener

`src/app/ranking/io/app_events/match_report_published_listener.rs` — même forme que `players::team_created_listener` : `init(app_event_bus, pool, repo, competition_port)`, `tokio::spawn` + boucle `rx.recv()`, désérialise `MatchReportAppEvent`, filtre sur la variante `MatchReportPublished`, construit `RecordMatchRankingCommand` depuis le payload (parsing `String` → VOs ici, pas dans le use case), appelle `record_match_ranking_use_case::execute`. En cas de `RulesNotConfigured` ou erreur repository : `tracing::error!`, event ignoré (pas de retry, cohérent avec les autres listeners du projet).

### `context.rs`

```rust
pub struct RankingContext {
    pub repository:       Arc<dyn IRankingRepository>,
    pub competition_port: Arc<dyn IRankingCompetitionPort>,
}

pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool, competition_port: Arc<dyn IRankingCompetitionPort>) {
    let repo: Arc<dyn IRankingRepository> = Arc::new(PgRankingRepository::new(pool.clone()));
    match_report_published_listener::init(app_event_bus, pool, repo, competition_port);
}

impl RankingContext {
    pub fn new(pool: &PgPool, competition_port: Arc<dyn IRankingCompetitionPort>) -> Self { ... }
}
```

### `router.rs` / `routes.rs`

`routes.rs` : `RANKING_CLASSEMENT_WIDGET = "/app/{space_id}/ranking/{competition_id}/{season_id}/widget"` (route déclarée maintenant, handler branché carte suivante — le router peut rester vide ou ne route rien encore, à ajuster selon ce qui compile le plus proprement avec la carte 197).

### Enregistrement (`app/mod.rs`, `state.rs`, `main.rs`)

Même schéma que `players` :
1. `pub mod ranking;` déjà fait (carte 192)
2. `pub ranking: RankingContext` dans `AppState`
3. Dans `main.rs::run_server`, avant la construction d'`AppState` :
   ```rust
   let ranking_competition_port = Arc::new(
       crate::infrastructure::ranking::competition_info_adapter::RankingCompetitionAdapter::new(
           Arc::new(crate::app::competitions::io::repository::season_repository::SeasonRepository::new(pool.clone())),
           competitions_team_info_port.clone(),  // ITeamInfoPort déjà construit plus haut dans main.rs
       ),
   );
   ranking::context::init_listeners(&app_event_bus, pool.clone(), ranking_competition_port.clone());
   ```
4. Dans le literal `AppState { ... }` : `ranking: RankingContext::new(&pool, ranking_competition_port.clone()), ...`
5. `.merge(app::ranking::router::router())` sur le router protégé (à côté de `.merge(app::players::router::router())`)

## Checklist

- [ ] `match_report_published_listener.rs` (init + handle_event, parsing payload → `RecordMatchRankingCommand`)
- [ ] `RankingContext` complet + `init_listeners`
- [ ] `pub ranking: RankingContext` dans `AppState`
- [ ] Câblage `main.rs` (adapter, `init_listeners` avant `AppState`, literal `AppState`, `.merge(router())`)
- [ ] Fake `RankingContext` ajouté dans le test-builder de `post_login.rs` (même pattern que les autres BCs)
- [ ] `cargo check --tests` passe
- [ ] `make check-arch` propre (axe 3 notamment — vérifier qu'aucun import direct de `teams` ne s'est glissé dans le listener)
- [ ] Test d'intégration : publier un `MatchReportPublished` sur un vrai `EventBus` → vérifier que 2 lignes apparaissent en base (pattern `test_player_match_impact_pipeline.rs`)
