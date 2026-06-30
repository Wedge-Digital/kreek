use crate::app::competitions::domain::competition_rules::CompetitionRules;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::common_types::SeasonId;
use std::collections::HashMap;

pub struct SaveCompetitionRulesCommand {
    pub season_id: SeasonId,
    pub season_name: String,
    pub rules: CompetitionRules,
}

#[derive(Debug)]
pub enum SaveCompetitionRulesError {
    RosterInMultipleTiers {
        roster: String,
        tiers: (String, String),
    },
    Database(String),
    SeasonNotFound,
}

impl From<SeasonRepositoryError> for SaveCompetitionRulesError {
    fn from(e: SeasonRepositoryError) -> Self {
        match e {
            SeasonRepositoryError::SeasonNotFound => Self::SeasonNotFound,
            other => Self::Database(other.to_string()),
        }
    }
}

pub async fn execute(
    cmd: SaveCompetitionRulesCommand,
    repo: &dyn ISeasonRepository,
) -> Result<(), SaveCompetitionRulesError> {
    let mut seen: HashMap<&str, &str> = HashMap::new();
    for tier in &cmd.rules.tiers {
        for roster in &tier.rosters {
            if let Some(prev) = seen.insert(roster.as_str(), tier.name.as_ref()) {
                return Err(SaveCompetitionRulesError::RosterInMultipleTiers {
                    roster: roster.clone(),
                    tiers: (prev.to_string(), tier.name.as_ref().to_string()),
                });
            }
        }
    }

    repo.save_rules(&cmd.season_id, &cmd.season_name, &cmd.rules)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_rules::{
        Activated, DefensiveBonus, OffensiveBonus, RankingPoints, RankingRules, TdDiff, TierRule,
    };
    use crate::app::shared_kernel::tier::{CreationBudget, StartingXp, TierName};
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{
        SeasonBaseInfo, SeasonRepositoryError,
    };
    use crate::app::shared_kernel::common_types::{CompetitionId, SeasonId};
    use async_trait::async_trait;

    struct FakeRepo {
        fail: bool,
    }

    #[async_trait]
    impl ISeasonRepository for FakeRepo {
        async fn save(&self, _: &CompetitionSeason) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_latest_season_id(
            &self,
            _: &CompetitionId,
        ) -> Result<Option<SeasonId>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_base_info(
            &self,
            _: &SeasonId,
        ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_rules(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionRules>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_rules(
            &self,
            _: &SeasonId,
            _: &str,
            _: &CompetitionRules,
        ) -> Result<(), SeasonRepositoryError> {
            if self.fail {
                Err(SeasonRepositoryError::Database("db error".into()))
            } else {
                Ok(())
            }
        }
        async fn find_structure(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionStructure>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_structure(
            &self,
            _: &SeasonId,
            _: &CompetitionStructure,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_invitations(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_invitations(
            &self,
            _: &SeasonId,
            _: &CompetitionInvitations,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn set_ready(&self, _: &SeasonId) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_full(
            &self,
            _: &SeasonId,
        ) -> Result<
            Option<crate::app::competitions::domain::season_repository_port::SeasonFull>,
            SeasonRepositoryError,
        > {
            Ok(None)
        }
    }

    fn base_rules(tiers: Vec<TierRule>) -> CompetitionRules {
        CompetitionRules {
            ranking_rules: RankingRules {
                win_points: RankingPoints::try_new(3).unwrap(),
                draw_points: RankingPoints::try_new(1).unwrap(),
                lose_points: RankingPoints::try_new(0).unwrap(),
                offensive_bonus: OffensiveBonus {
                    activated: Activated(true),
                    diff_td: TdDiff::try_new(3).unwrap(),
                    points: RankingPoints::try_new(1).unwrap(),
                },
                defensive_bonus: DefensiveBonus {
                    activated: Activated(true),
                    points: RankingPoints::try_new(1).unwrap(),
                },
                additionnal_ranking_points: HashMap::new(),
            },
            tiers,
        }
    }

    fn tier(name: &str, rosters: Vec<&str>) -> TierRule {
        TierRule {
            name: TierName::try_new(name).unwrap(),
            budget: CreationBudget(1060),
            starting_xp: StartingXp::try_new(0).unwrap(),
            rosters: rosters.into_iter().map(String::from).collect(),
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
            SaveCompetitionRulesCommand {
                season_id: SeasonId::new(),
                season_name: "Saison 1".into(),
                rules,
            },
            &FakeRepo { fail: false },
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn roster_dans_deux_tiers_retourne_erreur() {
        let rules = base_rules(vec![
            tier("Tier 1", vec!["HUMAN", "ORC"]),
            tier("Tier 2", vec!["DWARF", "HUMAN"]),
        ]);
        let result = execute(
            SaveCompetitionRulesCommand {
                season_id: SeasonId::new(),
                season_name: "Saison 1".into(),
                rules,
            },
            &FakeRepo { fail: false },
        )
        .await;
        assert!(matches!(
            result,
            Err(SaveCompetitionRulesError::RosterInMultipleTiers { roster, .. }) if roster == "HUMAN"
        ));
    }

    #[tokio::test]
    async fn erreur_bdd_remontee() {
        let rules = base_rules(vec![tier("Tier 1", vec!["HUMAN"])]);
        let result = execute(
            SaveCompetitionRulesCommand {
                season_id: SeasonId::new(),
                season_name: "Saison 1".into(),
                rules,
            },
            &FakeRepo { fail: true },
        )
        .await;
        assert!(matches!(
            result,
            Err(SaveCompetitionRulesError::Database(_))
        ));
    }
}
