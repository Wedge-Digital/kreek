# `match_report` — Publisher : app events de compensation

**Priorité : haute**
**Dépend de :** `229-mrc-use-case-handler.md`
**Fichiers :** `src/app/match_report/io/app_events/app_event_publisher.rs`, `src/app/shared_kernel/app_events/{match_report_app_events,player_match_impact_app_events}.rs`
**Spec :** `docs/specs/match-report-correction/recap/03-back.md`, `04-dtos.md`

## Objectif

Convertir le domain event `MatchReportUnpublished` en app events de
compensation. Carte pivot : sans elle, aucune des cartes 232 à 235 ne peut se
déclencher.

## Conception

### Payloads

```rust
// match_report_app_events.rs
MatchReportUnpublished(MatchReportUnpublishedPayload)

pub struct MatchReportUnpublishedPayload {
    pub match_report_id: String,
    pub space_id:        String,
    pub competition_id:  String,
    pub season_id:       String,
    pub round_id:        String,
    pub pairing_id:      Option<String>,
    pub home_team_id:    String,
    pub away_team_id:    String,
    pub unpublished_at:  DateTime<Utc>,
}
```

```rust
// player_match_impact_app_events.rs
TeamMatchImpactReverted { team_id: String, match_report_id: String },
```

**Identifiants seulement, aucune action.** Chaque BC défait ce qu'il a lui-même
enregistré, via son instantané dérivé — il ne recalcule rien depuis le payload.
C'est ce qui rend la compensation exacte même si le payload dérivait.

`TeamMatchImpactReverted` vit dans `player_match_impact_app_events` et non dans
`match_report_app_events` : le BC `players` ne connaît ni compétition ni saison,
son contrat doit rester étroit.

Ajouter les constantes `event_type()` correspondantes dans les deux enums.

### Publisher — piège à traiter

Le publisher filtre aujourd'hui sur `MatchReportPublished`, relit l'agrégat et
**exige l'état `Published`** ; tout autre état produit un `warn!` et rien
d'autre.

Après une dépublication, la relecture trouve `ReadyToPublish`. Sans adaptation,
**le publisher n'émettrait rien, en silence** — et toutes les compensations
resteraient lettre morte.

La relecture doit donc accepter l'état attendu **selon l'événement traité** :
`Published` pour une publication, `ReadyToPublish` pour une dépublication.

Émissions pour une dépublication :

1. `MatchReportAppEvent::MatchReportUnpublished(payload)` — 1
2. `PlayerMatchImpactAppEvent::TeamMatchImpactReverted` — 2, une par équipe

Découpage (20 lignes) : une fonction de construction de payload par cas, comme
`build_published_payload` aujourd'hui.

## Checklist

- [ ] `MatchReportUnpublishedPayload` + variant + `event_type()`
- [ ] `TeamMatchImpactReverted` + `event_type()` + `match_report_id()`
- [ ] Le publisher traite `MatchReportUnpublished` en plus de `MatchReportPublished`
- [ ] La relecture accepte `ReadyToPublish` pour une dépublication
- [ ] Les 3 app events émis (1 + 2)
- [ ] Test : une dépublication émet bien 1 + 2 app events
- [ ] Test : le payload ne contient aucune action
- [ ] Test de non-régression : une publication émet toujours ce qu'elle émettait
- [ ] `make test` passe
- [ ] `make check-arch` passe
