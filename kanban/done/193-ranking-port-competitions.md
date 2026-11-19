# BC `ranking` — Port ACL vers `competitions`

**Priorité : haute**
**Dépend de :** `192-ranking-domaine.md`
**Contexte :** `ranking/ports.rs` + `infrastructure/ranking/`
**Spec :** `docs/specs/ranking/classement/03-back.md`, `04-dtos.md`

## Objectif

`ranking` ne parle jamais directement à `teams` — uniquement à `competitions`, qui ré-expose ce dont `ranking` a besoin (règles de classement + équipes inscrites) via son propre port existant vers `teams` (`ITeamInfoPort`).

## Conception

`src/app/ranking/ports.rs` (nouveau fichier) :

```rust
pub struct RankingRulesInfo {
    pub win_points:  u32,
    pub draw_points: u32,
    pub lose_points: u32,
}

pub struct EnrolledTeamInfo {
    pub team_id:   String,
    pub team_name: String,
}

#[async_trait]
pub trait IRankingCompetitionPort: Send + Sync {
    async fn find_ranking_rules(&self, season_id: &str) -> Option<RankingRulesInfo>;
    async fn find_enrolled_teams(&self, season_id: &str) -> Vec<EnrolledTeamInfo>;
}
```

`src/infrastructure/ranking/competition_info_adapter.rs` (nouveau) :

```rust
pub struct RankingCompetitionAdapter {
    season_repo: Arc<dyn ISeasonRepository>,   // find_rules(season_id) → CompetitionRules.ranking_rules
    team_info_port: Arc<dyn ITeamInfoPort>,    // find_enrolled_teams(season_id) → déjà existant, competitions → teams
}
```

`find_ranking_rules` délègue à `season_repo.find_rules(season_id)` et mappe `CompetitionRules.ranking_rules.{win,draw,lose}_points` vers `RankingRulesInfo`. `find_enrolled_teams` délègue directement à `team_info_port.find_enrolled_teams(season_id)` et mappe `TeamInfoDto` (competitions) vers `EnrolledTeamInfo` (ranking) — jamais le DTO de `competitions` exposé tel quel dans `ranking`.

Pas de câblage dans `AppState`/`main.rs` à ce stade (carte 196) — ce module compile et se teste isolément (mocks du port).

## Checklist

- [ ] `IRankingCompetitionPort` + `RankingRulesInfo` + `EnrolledTeamInfo` dans `ranking/ports.rs`
- [ ] `infrastructure/ranking/mod.rs` + `competition_info_adapter.rs`
- [ ] `RankingCompetitionAdapter` mappe `CompetitionRules.ranking_rules` → `RankingRulesInfo` et `TeamInfoDto` → `EnrolledTeamInfo`
- [ ] Aucun import de `crate::app::teams` dans `ranking/` (seulement `competitions`, via `ITeamInfoPort`)
- [ ] `cargo check` passe
- [ ] `make check-arch` : axe 3 propre pour `ranking`
