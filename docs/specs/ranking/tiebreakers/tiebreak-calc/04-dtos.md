# Phase 4 — Contrats de données (`tiebreak-calc`)

Aucun contrat HTTP : l'unité n'expose ni endpoint ni formulaire. Les contrats concernés
sont la commande applicative, les types domaine, les DTOs de port et la projection.

## Commande — regroupement des stats par équipe

`RecordMatchRankingCommand` porte aujourd'hui **12 champs**, dont quatre par paire
symétrique (`home_score`/`away_score`, `home_casualties_inflicted`/…). Ajouter les
fautes et les réussites la porterait à 16, avec deux nouvelles paires.

Les stats d'une équipe sont regroupées, comme l'a été l'identité de la ligne dans
`MatchContext` lors de la feature `ranking-bonus-points` :

```rust
pub struct TeamMatchStats {
    pub score:       MatchScore,
    pub casualties:  CasualtiesInflicted,
    pub fouls:       FoulsCommitted,
    pub completions: CompletionsMade,
}

pub struct RecordMatchRankingCommand {
    pub competition_id:  CompetitionId,
    pub season_id:       SeasonId,
    pub round_id:        RoundId,
    pub match_report_id: MatchReportId,
    pub home_team_id:    TeamId,
    pub away_team_id:    TeamId,
    pub home:            TeamMatchStats,
    pub away:            TeamMatchStats,
    pub published_at:    DateTime<Utc>,
}
```

Bénéfice au-delà du nombre de champs : `record_home` / `record_away` construisent leurs
`MatchStats` en **croisant deux structs** au lieu de piocher champ par champ dans huit
champs à préfixe — c'est là que se glisse une erreur de symétrie (utiliser `home_score`
pour l'équipe away).

## Types domaine

### Value objects nouveaux

```rust
pub struct FoulsCommitted(pub u32);      // actions Agression
pub struct CompletionsMade(pub u32);     // actions Passe
pub struct TdFor(pub u32);
pub struct TdAgainst(pub u32);
pub struct CasualtiesTotal(pub u32);     // cumul des Sortie (≠ CasualtiesInflicted, par match)
pub struct Rank(pub u32);
```

`CasualtiesInflicted` existe déjà et vaut **par match** ; les compteurs cumulés sont des
types distincts pour que le compilateur empêche de confondre « sorties de ce match » et
« sorties de la saison ».

### `MatchStats` — deux champs de plus

```rust
pub struct MatchStats {
    pub own_td:               MatchScore,
    pub opponent_td:          MatchScore,
    pub casualties_inflicted: CasualtiesInflicted,
    pub fouls:                FoulsCommitted,      // nouveau
    pub completions:          CompletionsMade,     // nouveau
}
```

Les bonus n'utilisent pas les deux nouveaux champs — ils restent alimentés pour les
compteurs cumulés (règle 12 : on accumule tout, indépendamment de l'activation).

### `CumulativeTotals` — cinq compteurs de plus

```rust
pub struct CumulativeTotals {
    // … existant : matches_played, wins, draws, losses, ranking_points, bonus_points
    pub td_for:      TdFor,
    pub td_against:  TdAgainst,
    pub casualties:  CasualtiesTotal,
    pub fouls:       FoulsCommitted,
    pub completions: CompletionsMade,
}
```

`ZERO` est complété. `diff_td` **n'est pas un champ** : il se dérive de `td_for` et
`td_against` (règle 13).

### `standings.rs` — types d'ordonnancement

```rust
pub struct TeamCounters { /* les 5 compteurs + wins, pour value_of */ }
pub struct TeamStanding { pub team_id: TeamId, pub totals: CumulativeTotals }
pub struct TiebreakOrder(Vec<TiebreakCriterion>);   // actifs, ordre de priorité
```

`TeamStanding` ne porte que `team_id` et `totals` : les compteurs vivent déjà dans
`CumulativeTotals`, un second conteneur les dupliquerait. `TeamCounters` disparaît donc
du plan de la phase 3 — `value_of` lit directement les totaux.

```rust
impl TiebreakCriterion {
    pub fn direction(&self) -> Direction;                    // Desc partout sauf NbTdConceded
    pub fn value_of(&self, totals: &CumulativeTotals) -> i64; // signé : diff_td peut être négatif
}

pub fn compare(a: &TeamStanding, b: &TeamStanding, order: &TiebreakOrder) -> Ordering
pub fn order_standings(standings: &mut [TeamStanding], order: &TiebreakOrder)
pub fn assign_ranks(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<Rank>
```

## DTOs de port

```rust
// ranking/ports.rs — DTOs de lecture, primitives assumées
pub struct RankingLineRow {
    // … existant + bonus_points
    pub td_for:      u32,
    pub td_against:  u32,
    pub casualties:  u32,
    pub fouls:       u32,
    pub completions: u32,
}

pub struct TiebreakSettingInfo { pub code: String, pub activated: bool }
// RankingRulesInfo : + pub tiebreakers: Vec<TiebreakSettingInfo>   // ordonnés par priorité
```

L'ordre du `Vec` **est** la priorité, comme côté `competitions` — aucun champ de rang.

## Projection

```sql
ALTER TABLE ranking_lines
    ADD COLUMN td_for      INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN td_against  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN casualties  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN fouls       INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN completions INTEGER NOT NULL DEFAULT 0;
```

Pas de colonne `diff_td` : dérivée. Pas de backfill — projet hors production, projection
rebuildable.

## View models

`ClassementRowVm` est **inchangé dans sa forme** : le champ `rank` reçoit simplement un
rang calculé par le domaine au lieu de `idx + 1`. Les colonnes de compteurs relèvent de
l'unité `detailed-standings`.

## Interfaces d'utilisation

| Type | Émis par | Consommé par |
|---|---|---|
| `TeamMatchStats` | `match_report_published_listener` (comptage du payload) | `record_match_ranking_use_case` → `MatchStats` |
| `MatchStats` | Use case (croisement home/away) | `RankingLine::record_match` |
| `CumulativeTotals` (+ compteurs) | `to_totals` (depuis `RankingLineRow`) | `record_match`, `value_of`, `compare` |
| `RankingLineRow` (+ compteurs) | Repository | `to_totals` (écriture) et `standings_service` (lecture) |
| `TiebreakSettingInfo` | `competition_info_adapter` | `to_domain_rules` → `TiebreakOrder` |
| `TeamStanding` / `Rank` | `standings_service` | `builders.rs` (mapping VM uniquement) |

## Règles métier — état

Aucune règle nouvelle. La phase précise la matérialisation de la règle 13 (`diff_td`
dérivé, ni champ domaine ni colonne) et de la règle 12 (les deux nouveaux compteurs sont
alimentés même quand aucun bonus ne les utilise).
