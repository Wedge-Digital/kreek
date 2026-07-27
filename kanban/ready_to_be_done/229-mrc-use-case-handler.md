# `match_report` — Use case de dépublication, handler POST et route

**Priorité : haute**
**Dépend de :** `226-mrc-autorisation-publication.md`, `228-mrc-garde-fou-ports.md`
**Fichiers :** `src/app/match_report/use_cases/unpublish_match_report_use_case.rs` (nouveau), `src/app/match_report/io/web/recap_controller.rs`, `src/app/match_report/{routes,router}.rs`
**Spec :** `docs/specs/match-report-correction/recap/05-use-cases.md`, `02-front.md`

## Objectif

Rendre la dépublication déclenchable par HTTP. À l'issue de cette carte, le
rapport repasse en `ReadyToPublish` — les compensations dans les autres BCs
arrivent plus tard (cartes 231 à 235).

## Conception

### Use case

```rust
pub async fn execute(
    cmd: UnpublishMatchReportCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
    bus: &EventBus,
) -> Result<(), UnpublishMatchReportError>
```

```rust
pub struct UnpublishMatchReportCommand {
    pub match_report_id: MatchReportId,
    pub unpublished_by:  CoachId,
}

pub enum UnpublishMatchReportError {
    NotFound,
    NotPublished,
    NotEligible(CorrectionBlocker),
    Repository(String),
}
```

Orchestration, strictement symétrique de `publish_match_report_use_case` :

1. `repo.find_by_id()` → exiger `Published`
2. `correction_eligibility_service::evaluate()`
3. `published.unpublish(cmd.unpublished_by, eligibility)`
4. `repo.append(&id, &event, rtp.version - 1)` — **même convention de version que `publish`**
5. `bus.send(event.to_enveloppe(&id))` — bus **interne** du BC

L'app event bus n'apparaît pas dans la signature : le publisher fait la
conversion (carte 231).

Découpage (20 lignes) : `execute` + `load_published(repo, id)`.

### Route

```
POST /app/{space_id}/match-report/{match_report_id}/recap/unpublish
```

Constante `MATCH_REPORT_RECAP_UNPUBLISH` dans `routes.rs`, méthode
`recap_unpublish(space_id, match_report_id)`, câblage dans `router.rs`.

### Handler

```rust
pub async fn post_unpublish(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

| Cas | Réponse |
|---|---|
| non connecté | `401` |
| id invalide | `400` |
| non autorisé | `403` |
| succès | `HX-Refresh: true` |
| `NotEligible(_)` | `HX-Refresh: true` |
| `NotFound` / `NotPublished` | `404` |
| `Repository(_)` | `500` |

`NotEligible` renvoie un **rafraîchissement, pas une erreur** : c'est la
traduction HTTP de la règle 9a. Le coach revient sur la page et y lit la raison
recalculée, à jour.

Droits identiques à la publication (règle 4) : réutiliser `is_authorized()`.

Découpage (20 lignes) : une fonction pour l'autorisation, une pour la traduction
du résultat.

## Checklist

- [ ] `unpublish_match_report_use_case` avec commande et erreurs
- [ ] Convention `version - 1` sur l'append, comme `publish`
- [ ] Émission sur le bus **interne** uniquement
- [ ] Route et constante ajoutées, câblées dans `router.rs`
- [ ] `post_unpublish` avec les 7 cas de réponse
- [ ] `NotEligible` → `HX-Refresh`, pas une page d'erreur
- [ ] Toutes les fonctions sous 20 lignes
- [ ] Test : dépublication d'un rapport non publié → `NotPublished`
- [ ] Test : rapport introuvable → `NotFound`
- [ ] Test : garde-fou bloquant → `NotEligible` porteur du blocker
- [ ] Test : succès → l'agrégat relu est en `ReadyToPublish`
- [ ] `make test` passe
- [ ] `make check-arch` passe
