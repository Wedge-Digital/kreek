# BC `ranking` — Use case `record_match_ranking`

**Priorité : haute**
**Dépend de :** `193-ranking-port-competitions.md`, `194-ranking-repository.md`
**Contexte :** `ranking/use_cases/`
**Spec :** `docs/specs/ranking/classement/05-use-cases.md`

## Objectif

Orchestrer l'enregistrement des 2 lignes de classement (domicile + extérieur) suite à un match publié — sans encore le brancher sur l'event bus (carte suivante). Testable isolément avec des doublures de port/repository.

## Conception

`src/app/ranking/use_cases/record_match_ranking_use_case.rs` :

```rust
pub struct RecordMatchRankingCommand {
    pub competition_id: CompetitionId,
    pub season_id:        SeasonId,
    pub round_id:          RoundId,
    pub match_report_id:   MatchReportId,
    pub home_team_id:      TeamId,
    pub away_team_id:      TeamId,
    pub home_score:         MatchScore,
    pub away_score:         MatchScore,
    pub published_at:       DateTime<Utc>,
}

pub enum RecordMatchRankingError {
    RulesNotConfigured,
    Repository(String),
}

pub async fn execute(
    cmd: RecordMatchRankingCommand,
    repo: &dyn IRankingRepository,
    competition_port: &dyn IRankingCompetitionPort,
) -> Result<(), RecordMatchRankingError> {
    // 1. rules = competition_port.find_ranking_rules(season_id) — None → RulesNotConfigured, aucune ligne écrite
    // 2. prev_home = repo.find_latest_line(season_id, home_team_id)
    //    prev_away = repo.find_latest_line(season_id, away_team_id)
    // 3. RankingLine::record_match(...) × 2 (home avec outcome dérivé de home/away scores, away avec l'inverse)
    // 4. repo.insert_lines(&[home_line, away_line]) — atomique
}
```

Aucune connaissance HTTP/event bus dans ce fichier — pur orchestrateur (charge, appelle le domaine, persiste), conforme à la règle CLAUDE.md « Responsabilités des couches ».

## Checklist

- [ ] `RecordMatchRankingCommand`, `RecordMatchRankingError`, `execute(...)`
- [ ] Aucune logique métier dans le use case (délègue entièrement à `RankingLine::record_match`/`derive_outcome`)
- [ ] Tests unitaires avec doublures `IRankingRepository`/`IRankingCompetitionPort` : règles absentes → `RulesNotConfigured` sans écriture, équipe sans historique (`previous = None`), équipe avec historique (cumul correct), les 2 lignes insérées en un seul appel `insert_lines`
- [ ] `cargo check` + `cargo test` passent
