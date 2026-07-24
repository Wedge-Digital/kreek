# Phase 5 — Use cases (post-match-bonus-calc)

Unité **sans nouveau use case** : on **étend** le use case existant
`record_match_ranking_use_case::execute`. Sa signature externe et ses erreurs restent
**inchangées** — seuls la commande et l'orchestration interne s'enrichissent.

## Use case concerné

`app/ranking/use_cases/record_match_ranking_use_case.rs`

```rust
pub async fn execute(
    cmd: RecordMatchRankingCommand,
    repo: &dyn IRankingRepository,
    competition_port: &dyn IRankingCompetitionPort,
) -> Result<(), RecordMatchRankingError>
```

Signature **inchangée**. Le use case reste un chef d'orchestre : il ne calcule aucun
bonus (ça vit dans le domaine, phase 6), il coordonne.

## Changements

### 1. Commande enrichie (cf. phase 4)

`RecordMatchRankingCommand` gagne les sorties infligées par équipe (value objects,
côté command) ; les scores sont déjà présents.

```rust
pub home_casualties_inflicted: CasualtiesInflicted, // nouveau
pub away_casualties_inflicted: CasualtiesInflicted, // nouveau
```

Émetteur de la commande : le listener `handle_published` (IO), qui compte les `Sortie`
(décision A, phase 3). Le use case ne connaît jamais les types du payload.

### 2. `to_domain_rules` mappe les 3 bonus

Étendre le mapping DTO port → domaine pour traduire chaque `BonusRuleInfo` en règle
domaine correspondante (cf. phase 4, « Cohérence mapping port → domaine ») :

```rust
fn to_domain_rules(info: RankingRulesInfo) -> RankingRules {
    RankingRules {
        win_points:  RankingPoints(info.win_points),
        draw_points: RankingPoints(info.draw_points),
        lose_points: RankingPoints(info.lose_points),
        offensive_bonus: OffensiveBonusRule {
            activated: BonusActivated(info.offensive.activated),
            min_td:    MinTd(info.offensive.threshold),
            points:    RankingPoints(info.offensive.points),
        },
        defensive_bonus: DefensiveBonusRule {
            activated:       BonusActivated(info.defensive.activated),
            max_td_conceded: MaxTdConceded(info.defensive.threshold),
            points:          RankingPoints(info.defensive.points),
        },
        aggressive_bonus: AggressiveBonusRule {
            activated:     BonusActivated(info.aggressive.activated),
            min_casualties: MinCasualties(info.aggressive.threshold),
            points:         RankingPoints(info.aggressive.points),
        },
    }
}
```

Le sens de `threshold` est levé par le champ source (offensive/defensive/aggressive) —
pas d'ambiguïté à la traduction.

### 3. Construction de 2 `MatchStats` (décision B, phase 3)

Le use case ne dérive **plus** l'outcome lui-même (`derive_outcome` migre dans
`record_match`, phase 6). Il construit deux `MatchStats`, scores et casualties croisés :

```rust
let home_stats = MatchStats {
    own_td:               cmd.home_score,
    opponent_td:          cmd.away_score,
    casualties_inflicted: cmd.home_casualties_inflicted,
};
let away_stats = MatchStats {
    own_td:               cmd.away_score,
    opponent_td:          cmd.home_score,
    casualties_inflicted: cmd.away_casualties_inflicted,
};
```

### 4. Appel `record_match` via `MatchContext` (4 args)

`record_match` passe de 9 → 4 arguments : les 6 champs d'identité sont bundlés dans un
`MatchContext` (domaine, phase 6), `outcome` est remplacé par `stats` (dérivé en
interne). Le `#[allow(clippy::too_many_arguments)]` **saute** (décision validée :
intégration du `MatchContext`).

Le use case construit deux contextes (home/away) partageant les champs
compétition/saison/round/match_report/date, `.clone()` sur le premier, déplacés sur le
second :

```rust
let home_ctx = MatchContext {
    team_id: cmd.home_team_id,
    competition_id: cmd.competition_id.clone(),
    season_id: cmd.season_id.clone(),
    round_id: cmd.round_id.clone(),
    match_report_id: cmd.match_report_id.clone(),
    recorded_at: cmd.published_at,
};
let away_ctx = MatchContext {
    team_id: cmd.away_team_id,
    competition_id: cmd.competition_id,
    season_id: cmd.season_id,
    round_id: cmd.round_id,
    match_report_id: cmd.match_report_id,
    recorded_at: cmd.published_at,
};

let home_line = RankingLine::record_match(home_previous, home_ctx, home_stats, &rules);
let away_line = RankingLine::record_match(away_previous, away_ctx, away_stats, &rules);
```

## Orchestration inchangée par ailleurs

- Chargement des règles via `competition_port.find_ranking_rules(...).map(to_domain_rules)`.
- `load_previous` ×2 (home/away).
- `repo.insert_lines(&[home_line, away_line])`.

## Erreurs

`RecordMatchRankingError` **inchangé** :

```rust
pub enum RecordMatchRankingError {
    RulesNotConfigured,
    Repository(String),
}
```

Aucune erreur applicative nouvelle : un bonus désactivé n'est pas une erreur, c'est 0
point (décision domaine, phase 6). Une règle absente reste `RulesNotConfigured`.

## Impacts tests (rappel — traités dans les cartes, phase 8)

- `FakeCompetitionPort` : renvoyer les nouveaux champs bonus de `RankingRulesInfo`.
- `sample_cmd` : fournir `home/away_casualties_inflicted`.
- Nouvelles assertions : points bonus cumulés selon activation/seuils.

## Règle métier à cette étape

Aucune nouvelle. Le calcul lui-même (comparaison de seuils, gate d'activation,
cumul) est spécifié en **phase 6 — domaine** (`RankingRules::bonus_points`).