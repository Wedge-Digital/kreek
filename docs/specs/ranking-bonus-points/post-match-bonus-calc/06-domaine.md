# Phase 6 — Domaine (post-match-bonus-calc)

Cœur métier de l'unité : le calcul des points bonus vit **dans le domaine ranking**
(`RankingLine` / `RankingRules`), jamais dans le use case ni le listener. Le use case
coordonne, le listener compte les sorties, le domaine décide « combien de points ? ».

## A. Récapitulatif exhaustif des règles métier (validé)

Les 3 bonus sont **cumulables**, s'**ajoutent** aux points V/N/D, sont **indépendants
du résultat** (une équipe qui perd peut les toucher), évalués **par équipe** sur un
match donné. **Un bonus n'est calculé que s'il est `activated`** ; désactivé ⇒ 0 point.

| Bonus | Condition (par équipe) | Comparateur | Donnée | Config |
|---|---|---|---|---|
| Offensif | TD marqués ≥ seuil | `≥` (large) | `own_td` | `activated`, `min_td`, `points` |
| Défensif | TD encaissés ≤ seuil | `≤` (large) | `opponent_td` | `activated`, `max_td_conceded`, `points` |
| Agressif | Sorties infligées **> Y** | `>` (**strict**) | `casualties_inflicted` | `activated`, `min_casualties`, `points` |

- « Sortie » = action `Sortie` **seule** (pas `Blesse`, pas `Agression`). Ce
  filtrage/comptage se fait dans le **listener (IO)** — le domaine reçoit un
  `CasualtiesInflicted` déjà compté.
- L'**outcome** (V/N/D) est dérivé des deux scores (`derive_outcome`, existant) et
  migre **à l'intérieur** de `record_match` (décision B, phase 3).
- Points totaux d'un match pour une équipe :
  `points_résultat(outcome) + Σ bonus activés remplis`.

## B. Value objects (style `pub` newtype, sans invariant — règle CQRS)

```rust
pub struct CasualtiesInflicted(pub u32);
pub struct MinTd(pub u32);
pub struct MaxTdConceded(pub u32);
pub struct MinCasualties(pub u32);
pub struct BonusActivated(pub bool);
```

Aucune borne de validité : ce sont des compteurs/seuils sans invariant à protéger
(même régime que `MatchScore`, `RankingPoints`, `MatchesPlayed`).

## C. Structs domaine

### `MatchStats` — remplace `outcome` en entrée de `record_match`

```rust
pub struct MatchStats {
    pub own_td: MatchScore,
    pub opponent_td: MatchScore,
    pub casualties_inflicted: CasualtiesInflicted,
}
```

### `MatchContext` — bundle des champs d'identité de la ligne (nouveau)

Regroupe les 6 arguments d'identité passés à `record_match` : fait passer la signature
de 9 → 4 arguments et **supprime le `#[allow(clippy::too_many_arguments)]`**.

```rust
#[derive(Debug, Clone)]
pub struct MatchContext {
    pub team_id: TeamId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub match_report_id: MatchReportId,
    pub recorded_at: DateTime<Utc>,
}
```

`Clone` requis : le use case construit deux contextes (home/away) partageant les
champs compétition/saison/round/match_report/date.

### Règles de bonus + extension `RankingRules`

```rust
pub struct OffensiveBonusRule  { pub activated: BonusActivated, pub min_td: MinTd,                 pub points: RankingPoints }
pub struct DefensiveBonusRule  { pub activated: BonusActivated, pub max_td_conceded: MaxTdConceded, pub points: RankingPoints }
pub struct AggressiveBonusRule { pub activated: BonusActivated, pub min_casualties: MinCasualties,  pub points: RankingPoints }

pub struct RankingRules {
    pub win_points: RankingPoints,
    pub draw_points: RankingPoints,
    pub lose_points: RankingPoints,
    pub offensive_bonus: OffensiveBonusRule,   // nouveau
    pub defensive_bonus: DefensiveBonusRule,   // nouveau
    pub aggressive_bonus: AggressiveBonusRule, // nouveau
}
```

## D. Méthodes domaine

Chaque règle **porte son propre comparateur** (méthode courte, < 20 lignes, le
comparateur vit avec la règle — pas de `if` géant dans l'agrégateur) :

```rust
impl OffensiveBonusRule {
    fn points_for(&self, stats: &MatchStats) -> RankingPoints {
        if self.activated.0 && u32::from(stats.own_td.0) >= self.min_td.0 {
            self.points
        } else { RankingPoints(0) }
    }
}
impl DefensiveBonusRule {
    fn points_for(&self, stats: &MatchStats) -> RankingPoints {
        if self.activated.0 && u32::from(stats.opponent_td.0) <= self.max_td_conceded.0 {
            self.points
        } else { RankingPoints(0) }
    }
}
impl AggressiveBonusRule {
    fn points_for(&self, stats: &MatchStats) -> RankingPoints {
        if self.activated.0 && stats.casualties_inflicted.0 > self.min_casualties.0 { // strict
            self.points
        } else { RankingPoints(0) }
    }
}
```

Agrégateur (reste trivial) :

```rust
impl RankingRules {
    pub fn bonus_points(&self, stats: &MatchStats) -> RankingPoints {
        self.offensive_bonus.points_for(stats)
            + self.defensive_bonus.points_for(stats)
            + self.aggressive_bonus.points_for(stats)
    }
}
```

`record_match` — nouvelle signature (4 args), dérive l'outcome en interne, ajoute les
bonus, **sans `#[allow]`** :

```rust
pub fn record_match(
    previous: Option<CumulativeTotals>,
    ctx: MatchContext,
    stats: MatchStats,
    rules: &RankingRules,
) -> RankingLine {
    let outcome = Self::derive_outcome(stats.own_td, stats.opponent_td);
    let CumulativeTotals { matches_played, wins, draws, losses, ranking_points: points } =
        previous.unwrap_or(CumulativeTotals::ZERO);
    let match_points = match outcome {
        MatchOutcome::Win  => rules.win_points,
        MatchOutcome::Draw => rules.draw_points,
        MatchOutcome::Loss => rules.lose_points,
    };
    RankingLine {
        team_id: ctx.team_id,
        competition_id: ctx.competition_id,
        season_id: ctx.season_id,
        round_id: ctx.round_id,
        match_report_id: ctx.match_report_id,
        recorded_at: ctx.recorded_at,
        matches_played: MatchesPlayed(matches_played.0 + 1),
        wins:   WinCount(wins.0   + u32::from(outcome == MatchOutcome::Win)),
        draws:  DrawCount(draws.0 + u32::from(outcome == MatchOutcome::Draw)),
        losses: LossCount(losses.0 + u32::from(outcome == MatchOutcome::Loss)),
        ranking_points: points + match_points + rules.bonus_points(&stats),
    }
}
```

`derive_outcome` reste tel quel (déjà dans le domaine).

## E. Erreurs domaine

**Aucune.** Un bonus désactivé ou dont la condition n'est pas remplie vaut 0 point —
ce n'est pas une violation d'invariant. Pas de nouvelle variante `DomainError`.

## F. Tests unitaires prévus (un par règle)

**Par bonus** (offensif / défensif / agressif) :
- activé + condition remplie → ajoute `points` ;
- activé + condition non remplie → 0 ;
- désactivé (même condition remplie) → 0.

**Frontières de comparateur** :
- offensif `own_td == min_td` (≥ ⇒ oui) ;
- défensif `opponent_td == max_td_conceded` (≤ ⇒ oui) ;
- agressif `casualties == min_casualties` (> ⇒ **non**) et `== min_casualties + 1` (⇒ oui).

**Composition** :
- cumul des 3 bonus + points de résultat sur un même match ;
- indépendance du résultat : une **défaite** remplissant un bonus le touche quand même.

**Régression** (signature `MatchContext`/`MatchStats`) :
- adapter les tests V/N/D existants (helper `ctx()` construisant un `MatchContext`,
  `stats(own, opp, cas)` construisant un `MatchStats`) ;
- vérifier que `derive_outcome` interne produit les mêmes V/N/D qu'avant.