# BC `teams` — Listener `MatchReportPublished` → phase `PlayerImprovement` (câblage minimal)

**Priorité : moyenne**
**Dépend de :** `169-teams-domain-dismissals-target-match-report-id.md`
**Contexte :** `teams` — app events (IO)

## Objectif

Faire entrer l'équipe en phase `PlayerImprovement` quand un rapport de match
est publié, pour que le bandeau "Phase d'amélioration" (carte 171) soit
atteignable en conditions réelles. Câblage **minimal** — le calcul réel des
revenus, du jet de fans dévoués et des gains SPP reste hors périmètre (cartes
35/145/154), ici les valeurs sont stubbées. Spec complète :
`docs/specs/team-state-management/team-detail/02-07-conception.md` (Phase 6-7).

---

## Conception

### Nouveau fichier `io/app_events/match_report_published_listener.rs`

Même structure que `match_report_confirmed_listener.rs` : souscrit à
`app_event_bus`, filtre sur `MatchReportAppEvent::MatchReportPublished`, traite
`home_team_id` et `away_team_id` indépendamment.

```rust
pub fn init(app_event_bus: &EventBus, team_repo: Arc<dyn ITeamRepository>) {
    let mut rx = app_event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(MatchReportAppEvent::MatchReportPublished(payload)) =
                        serde_json::from_value(envelope.payload.clone())
                    else { continue };

                    handle_team(&team_repo, &payload.home_team_id, derive_result(payload.home_score, payload.away_score)).await;
                    handle_team(&team_repo, &payload.away_team_id, derive_result(payload.away_score, payload.home_score)).await;
                }
                Err(RecvError::Lagged(n)) => tracing::warn!("match_report_published_listener: lagged by {n}"),
                Err(RecvError::Closed) => break,
            }
        }
    });
}

fn derive_result(own_score: u8, opponent_score: u8) -> MatchResult {
    match own_score.cmp(&opponent_score) {
        Ordering::Greater => MatchResult::Win,
        Ordering::Equal   => MatchResult::Draw,
        Ordering::Less    => MatchResult::Loss,
    }
}

async fn handle_team(team_repo: &Arc<dyn ITeamRepository>, team_id: &str, result: MatchResult) {
    let Ok(Some(team)) = team_repo.find_by_id(team_id).await else {
        tracing::warn!("match_report_published_listener: team {team_id} not found or error");
        return;
    };
    match team.start_post_match_sequence(result, 0, Kpo(0), vec![]) {
        Ok(event) => {
            if let Err(e) = team_repo.append(team_id, &event, team.version).await {
                tracing::error!("match_report_published_listener: append {team_id}: {e}");
            }
        }
        Err(e) => tracing::warn!("match_report_published_listener: start_post_match_sequence {team_id}: {e}"),
    }
}
```

`start_post_match_sequence` échoue déjà proprement (`DomainError::WrongGamePhase`)
si l'équipe n'est pas en `ReadyToPlay` (rapport déjà traité, replay du listener,
etc.) — on logue et on ignore, jamais de panic. Même pattern que
`match_report_confirmed_listener`.

### Câblage (`context.rs`)

```rust
pub fn init_listeners(app_event_bus: &EventBus, pool: PgPool) {
    let repo = Arc::new(TeamRepository::new(pool));
    team_created_listener::init(app_event_bus, repo.clone());
    match_report_confirmed_listener::init(app_event_bus, repo.clone());
    match_report_published_listener::init(app_event_bus, repo);   // nouveau
}
```

---

## Checklist

- [ ] `match_report_published_listener.rs` créé, souscrit à `MatchReportPublished`
- [ ] Dérivation `MatchResult` depuis les scores, indépendamment pour chaque équipe
- [ ] Erreur domaine (équipe hors `ReadyToPlay`) → log warn, pas de panic
- [ ] Câblé dans `context::init_listeners`
- [ ] Test : dérivation correcte de `MatchResult` (victoire/nul/défaite) selon les scores
- [ ] Test : équipe introuvable ou hors phase → ignoré proprement (pas de crash du listener)
