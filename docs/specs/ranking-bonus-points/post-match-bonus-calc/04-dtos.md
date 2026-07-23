# Phase 4 — Contrats de données (post-match-bonus-calc)

## DTO de port `RankingRulesInfo` (query, primitifs)

Les 3 bonus ont la même forme (activation + 1 seuil + points) → sous-DTO générique
`BonusRuleInfo`. Primitifs acceptés (DTO de lecture, règle CQRS).

```rust
pub struct BonusRuleInfo {
    pub activated: bool,
    pub threshold: u32,   // min_td (off) | max_td_conceded (def) | min_casualties (agg)
    pub points: u32,
}

pub struct RankingRulesInfo {
    pub win_points: u32,
    pub draw_points: u32,
    pub lose_points: u32,
    pub offensive: BonusRuleInfo,
    pub defensive: BonusRuleInfo,
    pub aggressive: BonusRuleInfo,
}
```

Le sens de `threshold` est porté par le champ qui le contient (offensive/defensive/
aggressive) — pas d'ambiguïté à la consommation.

## DTO d'entrée (command)

`RecordMatchRankingCommand` — ajout des sorties infligées par équipe (value objects,
côté command). Les scores sont déjà présents.

```rust
pub home_score: MatchScore,                        // existant
pub away_score: MatchScore,                        // existant
pub home_casualties_inflicted: CasualtiesInflicted, // nouveau (VO)
pub away_casualties_inflicted: CasualtiesInflicted, // nouveau (VO)
```

## Value objects & structs domaine (ranking)

Style `pub` newtype existant (`MatchScore(pub u8)`, `RankingPoints(pub u32)`), pas de
nutype dans ce BC. Bornes de validité : aucune (compteurs/seuils sans invariant —
détails Phase 6).

```rust
pub struct CasualtiesInflicted(pub u32);
pub struct MinTd(pub u32);
pub struct MaxTdConceded(pub u32);
pub struct MinCasualties(pub u32);
pub struct BonusActivated(pub bool);

/// Stats d'une équipe sur le match — remplace `outcome` en entrée de record_match
/// (l'outcome est dérivé en interne via derive_outcome).
pub struct MatchStats {
    pub own_td: MatchScore,
    pub opponent_td: MatchScore,
    pub casualties_inflicted: CasualtiesInflicted,
}

pub struct OffensiveBonusRule {
    pub activated: BonusActivated,
    pub min_td: MinTd,
    pub points: RankingPoints,
}
pub struct DefensiveBonusRule {
    pub activated: BonusActivated,
    pub max_td_conceded: MaxTdConceded,
    pub points: RankingPoints,
}
pub struct AggressiveBonusRule {
    pub activated: BonusActivated,
    pub min_casualties: MinCasualties,
    pub points: RankingPoints,
}

// RankingRules (ranking) : + offensive_bonus / defensive_bonus / aggressive_bonus
```

## Interfaces d'utilisation (émetteur → consommateur)

| DTO | Émetteur | Consommateur |
|---|---|---|
| `RankingRulesInfo` (+ `BonusRuleInfo`) | adapter `find_ranking_rules` (infra) | use case `to_domain_rules` |
| `RecordMatchRankingCommand` (+ casualties) | listener `handle_published` (IO) | use case `execute` |
| `MatchStats` | use case `execute` (construit depuis la commande) | `record_match` (domaine) |
| ranking `RankingRules` (+ bonus rules) | `to_domain_rules` (use case) | `record_match` / `bonus_points` (domaine) |

## Cohérence mapping port → domaine

`to_domain_rules` traduit `BonusRuleInfo` → règle domaine correspondante :
- `offensive.threshold` → `MinTd`
- `defensive.threshold` → `MaxTdConceded`
- `aggressive.threshold` → `MinCasualties`
- `activated`/`points` → `BonusActivated` / `RankingPoints`

## Règle métier à cette étape

Aucune nouvelle.
