use std::collections::HashMap;
use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, ICompetitionRepository};
use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::shared_kernel::common_types::CompetitionId;

pub struct SaveCompetitionRulesCommand {
    pub competition_id: CompetitionId,
    pub rules:          CompetitionRules,
}

#[derive(Debug)]
pub enum SaveCompetitionRulesError {
    RosterInMultipleTiers { roster: String, tiers: (String, String) },
    Database(String),
    CompetitionNotFound,
}

impl From<CompetitionRepositoryError> for SaveCompetitionRulesError {
    fn from(e: CompetitionRepositoryError) -> Self {
        match e {
            CompetitionRepositoryError::CompetitionNotFound => Self::CompetitionNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

pub async fn execute(
    cmd: SaveCompetitionRulesCommand,
    repo: &dyn ICompetitionRepository,
) -> Result<(), SaveCompetitionRulesError> {
    // Un roster ne peut pas apparaître dans deux tiers différents.
    let mut seen: HashMap<&str, &str> = HashMap::new();
    for tier in &cmd.rules.tiers {
        for roster in &tier.rosters {
            if let Some(prev) = seen.insert(roster.as_str(), tier.name.as_str()) {
                return Err(SaveCompetitionRulesError::RosterInMultipleTiers {
                    roster: roster.clone(),
                    tiers:  (prev.to_string(), tier.name.clone()),
                });
            }
        }
    }

    repo.save_rules(&cmd.competition_id, &cmd.rules).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use crate::app::competitions::domain::competition::Competition;
    use crate::app::competitions::domain::competition_repository_port::{CompetitionRepositoryError, CompetitionSummary};
    use crate::app::competitions::domain::competition_rules::{DefensiveBonus, OffensiveBonus, RankingRules, TierRule};
    use crate::app::shared_kernel::common_types::SpaceId;
    use crate::app::shared_kernel::competition_name::CompetitionName;

    struct FakeRepo { fail: bool }

    #[async_trait]
    impl ICompetitionRepository for FakeRepo {
        async fn name_exists_in_space(&self, _: &CompetitionName, _: &SpaceId) -> Result<bool, CompetitionRepositoryError> { Ok(false) }
        async fn save(&self, _: &Competition) -> Result<(), CompetitionRepositoryError> { Ok(()) }
        async fn find_by_space_id(&self, _: &SpaceId) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> { Ok(vec![]) }
        async fn save_rules(&self, _: &CompetitionId, _: &CompetitionRules) -> Result<(), CompetitionRepositoryError> {
            if self.fail { Err(CompetitionRepositoryError::Database("db error".into())) } else { Ok(()) }
        }
        async fn find_rules(&self, _: &CompetitionId) -> Result<Option<CompetitionRules>, CompetitionRepositoryError> {
            Ok(None)
        }
    }

    fn base_rules(tiers: Vec<TierRule>) -> CompetitionRules {
        CompetitionRules {
            ranking_rules: RankingRules {
                win_points:  3,
                draw_points: 1,
                lose_points: 0,
                offensive_bonus: OffensiveBonus { activated: true, diff_td: 3, points: 1 },
                defensive_bonus: DefensiveBonus { activated: true, points: 1 },
                additionnal_ranking_points: HashMap::new(),
            },
            tiers,
        }
    }

    fn tier(name: &str, rosters: Vec<&str>) -> TierRule {
        TierRule {
            name:        name.into(),
            budget:      1060,
            starting_xp: 0,
            rosters:     rosters.into_iter().map(String::from).collect(),
            inducements: vec![],
            star_players: vec![],
        }
    }

    #[tokio::test]
    async fn succes_sans_conflit() {
        let rules = base_rules(vec![
            tier("Tier 1", vec!["HUMAN", "ORC"]),
            tier("Tier 2", vec!["DWARF", "ELF"]),
        ]);
        let result = execute(
            SaveCompetitionRulesCommand { competition_id: CompetitionId::new(), rules },
            &FakeRepo { fail: false },
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn roster_dans_deux_tiers_retourne_erreur() {
        let rules = base_rules(vec![
            tier("Tier 1", vec!["HUMAN", "ORC"]),
            tier("Tier 2", vec!["DWARF", "HUMAN"]),
        ]);
        let result = execute(
            SaveCompetitionRulesCommand { competition_id: CompetitionId::new(), rules },
            &FakeRepo { fail: false },
        ).await;
        assert!(matches!(
            result,
            Err(SaveCompetitionRulesError::RosterInMultipleTiers { roster, .. }) if roster == "HUMAN"
        ));
    }

    #[tokio::test]
    async fn erreur_bdd_remontee() {
        let rules = base_rules(vec![tier("Tier 1", vec!["HUMAN"])]);
        let result = execute(
            SaveCompetitionRulesCommand { competition_id: CompetitionId::new(), rules },
            &FakeRepo { fail: true },
        ).await;
        assert!(matches!(result, Err(SaveCompetitionRulesError::Database(_))));
    }
}
