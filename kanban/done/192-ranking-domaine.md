# BC `ranking` — Domaine (RankingLine, calcul des points de classement)

**Priorité : haute**
**Dépend de :** rien
**Contexte :** nouveau BC `ranking` — domaine pur, aucune dépendance framework
**Spec :** `docs/specs/ranking/classement/06-domaine.md`

## Objectif

Poser le cœur métier du BC `ranking` : la construction d'une ligne de classement à partir de la ligne précédente d'une équipe (ou son absence), du résultat d'un match, et des règles de classement de la compétition. Calcul **total** — aucune erreur domaine possible, entièrement testable sans I/O.

## Conception

Nouveau fichier `src/app/ranking/mod.rs` (déclare `pub mod domain;` uniquement pour l'instant — pas encore de `ports`/`context`/`router`, ajoutés par les cartes suivantes) et `src/app/ranking/domain/ranking_line.rs`.

```rust
pub struct MatchScore(u8);       // newtype, pas de nutype (aucun invariant)
pub struct RankingPoints(u32);   // newtype, Add dérivé/implémenté

pub struct RankingRules {
    pub win_points:  RankingPoints,
    pub draw_points:  RankingPoints,
    pub lose_points:  RankingPoints,
}

pub enum MatchOutcome { Win, Draw, Loss }

pub struct RankingLine {
    pub team_id:          crate::app::shared_kernel::team::TeamId,
    pub season_id:        crate::app::shared_kernel::common_types::SeasonId,
    pub round_id:          crate::app::shared_kernel::common_types::RoundId,
    pub match_report_id:   crate::app::shared_kernel::common_types::MatchReportId,
    pub recorded_at:        chrono::DateTime<chrono::Utc>,
    pub matches_played:     u32,
    pub wins:                u32,
    pub draws:               u32,
    pub losses:              u32,
    pub ranking_points:      RankingPoints,
}

impl RankingLine {
    pub fn derive_outcome(own_score: MatchScore, opponent_score: MatchScore) -> MatchOutcome { /* cf. spec */ }
    pub fn record_match(previous: Option<&RankingLine>, /* ...ids, recorded_at */, outcome: MatchOutcome, rules: &RankingRules) -> RankingLine { /* cf. spec */ }
}
```

Déclarer `pub mod ranking;` dans `src/app/mod.rs` (module compilé, pas encore utilisé ailleurs — normal à ce stade, pas de warning bloquant).

## Checklist

- [ ] `src/app/ranking/mod.rs`, `src/app/ranking/domain/mod.rs`, `src/app/ranking/domain/ranking_line.rs`
- [ ] `MatchScore`, `RankingPoints` (newtypes, pas de primitif nu)
- [ ] `RankingRules`, `MatchOutcome`, `RankingLine`
- [ ] `RankingLine::derive_outcome` + `RankingLine::record_match`
- [ ] Tests unitaires (cf. tableau Phase 6) : dérivation résultat (3 cas), 2 lignes symétriques par match, cumul depuis une ligne existante, première ligne (previous=None), cumul sur 3 appels successifs, points appliqués selon l'issue
- [ ] `pub mod ranking;` dans `app/mod.rs`
- [ ] `cargo check` + `cargo test` passent
- [ ] `make check-arch` : axe 2 (pureté domaine) passe pour `ranking/domain/`
