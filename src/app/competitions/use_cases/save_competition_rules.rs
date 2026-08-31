use crate::app::competitions::domain::competition_rules::{CompetitionRules, TiebreakConfig};
use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::competitions::ports::ITiebreakCatalogPort;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use std::collections::HashSet;

#[derive(Debug)]
pub struct SaveCompetitionRulesCommand {
    pub season_id: SeasonId,
    pub season_name: String,
    pub rules: CompetitionRules,
}

#[derive(Debug)]
pub enum SaveCompetitionRulesError {
    /// La règle vit désormais dans le domaine (`CompetitionRules::
    /// ensure_roster_unicity`, carte 417). La variante reste ici parce que
    /// c'est la surface d'erreur du use case ; seule sa **source** a changé.
    RosterInMultipleTiers {
        roster: String,
        tiers: (String, String),
    },
    /// Code de départage absent du catalogue possédé par le BC `ranking`.
    UnknownTiebreakCriterion {
        code: String,
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

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: SaveCompetitionRulesCommand,
    repo: &dyn ISeasonRepository,
    catalog: &dyn ITiebreakCatalogPort,
) -> Result<(), SaveCompetitionRulesError> {
    // Traduction explicite plutôt qu'un `From` : le use case n'a qu'une seule
    // erreur domaine à relayer, et un `From` général laisserait passer les
    // quatre autres variantes sous ce même nom — un doublon de poule
    // s'annoncerait « roster dans deux tiers ».
    if let Err(DomainError::RosterInMultipleTiers { roster, tiers }) =
        cmd.rules.ensure_roster_unicity()
    {
        return Err(SaveCompetitionRulesError::RosterInMultipleTiers { roster, tiers });
    }
    ensure_known_tiebreak_codes(&cmd.rules.ranking_rules.tiebreakers, catalog)?;

    repo.save_rules(&cmd.season_id, &cmd.season_name, &cmd.rules)
        .await?;
    Ok(())
}

/// Vérifie que chaque code soumis existe au catalogue, possédé par le BC
/// `ranking`. L'exhaustivité n'est **pas** exigée : une configuration qui omet un
/// critère du catalogue est valide, le formulaire la complétera à l'hydratation.
fn ensure_known_tiebreak_codes(
    config: &TiebreakConfig,
    catalog: &dyn ITiebreakCatalogPort,
) -> Result<(), SaveCompetitionRulesError> {
    let known: HashSet<String> = catalog.all().into_iter().map(|c| c.code).collect();
    for setting in config.settings() {
        let code = setting.code.as_ref();
        if !known.contains(code) {
            return Err(SaveCompetitionRulesError::UnknownTiebreakCriterion {
                code: code.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
    use crate::app::competitions::domain::competition_rules::{
        Activated, AggressiveBonus, DefensiveBonus, MaxTdConceded, MinCasualties, MinTd,
        OffensiveBonus, RankingPoints, RankingRules, TiebreakCode, TierRule,
    };
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{
        SeasonBaseInfo, SeasonRepositoryError,
    };
    use crate::app::competitions::ports::TiebreakCriterionDto;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::tier::{CreationBudget, StartingXp, TierName};
    use async_trait::async_trait;

    struct FakeRepo {
        fail: bool,
    }

    #[async_trait]
    impl ISeasonRepository for FakeRepo {
        /// Doublure : le contrôle d'appartenance est exercé par les tests de
        /// handler, sur une vraie base.
        async fn find_space_id(
            &self,
            _: &SeasonId,
        ) -> Result<Option<String>, SeasonRepositoryError> {
            Ok(None)
        }

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
        async fn save_rules_keep_status(
            &self,
            _: &SeasonId,
            _: &str,
            _: &CompetitionRules,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
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
        /// Hors du périmètre de ce use case : la doublure échoue bruyamment
        /// si on l'y appelle, plutôt que de rendre un `Ok` qui ferait passer au
        /// vert un test n'ayant rien exercé.
        async fn save_structure_and_prune_groups(
            &self,
            _: &SeasonId,
            _: &CompetitionStructure,
            _: &[String],
        ) -> Result<u64, SeasonRepositoryError> {
            unimplemented!("hors du périmètre de ce use case")
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
            _: &CompetitionNotifications,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn save_visibility(
            &self,
            _: &SeasonId,
            _: &CompetitionInvitations,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_notifications(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionNotifications>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_notifications(
            &self,
            _: &SeasonId,
            _: &CompetitionNotifications,
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
                    min_td: MinTd::try_new(3).unwrap(),
                    points: RankingPoints::try_new(1).unwrap(),
                },
                defensive_bonus: DefensiveBonus {
                    activated: Activated(true),
                    points: RankingPoints::try_new(1).unwrap(),
                    max_td_conceded: MaxTdConceded::try_new(1).unwrap(),
                },
                aggressive_bonus: AggressiveBonus {
                    activated: Activated(false),
                    points: RankingPoints::try_new(1).unwrap(),
                    min_casualties: MinCasualties::try_new(2).unwrap(),
                },
                tiebreakers: tiebreakers(&["diff_td", "nb_td"]),
            },
            tiers,
        }
    }

    fn tiebreakers(codes: &[&str]) -> TiebreakConfig {
        let codes = codes
            .iter()
            .map(|c| TiebreakCode::try_new(*c).unwrap())
            .collect();
        TiebreakConfig::all_active(codes).unwrap()
    }

    /// Catalogue de test : les 7 codes réels, comme les expose l'adapter.
    struct FakeCatalog;

    impl ITiebreakCatalogPort for FakeCatalog {
        fn all(&self) -> Vec<TiebreakCriterionDto> {
            [
                "diff_td",
                "nb_td",
                "nb_td_conceded",
                "nb_cas",
                "nb_wins",
                "nb_fouls",
                "nb_reu",
            ]
            .into_iter()
            .map(|code| TiebreakCriterionDto {
                code: code.to_string(),
                label: format!("libellé {code}"),
            })
            .collect()
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
            &FakeCatalog,
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
            &FakeCatalog,
        )
        .await;
        assert!(matches!(
            result,
            Err(SaveCompetitionRulesError::RosterInMultipleTiers { roster, .. }) if roster == "HUMAN"
        ));
    }

    /// Enregistre des règles dont seule la configuration de départage varie.
    async fn save_with_tiebreakers(codes: &[&str]) -> Result<(), SaveCompetitionRulesError> {
        let mut rules = base_rules(vec![tier("Tier 1", vec!["HUMAN"])]);
        rules.ranking_rules.tiebreakers = tiebreakers(codes);
        execute(
            SaveCompetitionRulesCommand {
                season_id: SeasonId::new(),
                season_name: "Saison 1".into(),
                rules,
            },
            &FakeRepo { fail: false },
            &FakeCatalog,
        )
        .await
    }

    #[tokio::test]
    async fn code_de_departage_hors_catalogue_retourne_erreur() {
        let result = save_with_tiebreakers(&["diff_td", "nb_cartons_rouges"]).await;
        assert!(matches!(
            result,
            Err(SaveCompetitionRulesError::UnknownTiebreakCriterion { code })
                if code == "nb_cartons_rouges"
        ));
    }

    /// L'exhaustivité n'est pas exigée : une configuration partielle est valide.
    #[tokio::test]
    async fn configuration_partielle_du_catalogue_est_acceptee() {
        assert!(save_with_tiebreakers(&["nb_cas"]).await.is_ok());
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
            &FakeCatalog,
        )
        .await;
        assert!(matches!(
            result,
            Err(SaveCompetitionRulesError::Database(_))
        ));
    }
}
