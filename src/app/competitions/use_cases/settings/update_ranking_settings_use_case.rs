//! Modifier le barème d'une saison **en cours**, et rejouer le classement.
//!
//! C'est le use case qui porte tout le risque de l'onglet Paramètres : changer
//! un barème sans rejouer produirait un classement qui mélange deux règles, et
//! personne ne l'apprendrait — les totaux resteraient plausibles.
//!
//! # Deux décisions qui se lisent mal sans leur raison
//!
//! **L'enregistrement précède le recalcul, jamais l'inverse.** Le rejeu lit le
//! barème par son propre port (`IRankingCompetitionPort::find_ranking_rules`) :
//! le lancer avant l'écriture le ferait rejouer avec l'**ancien** barème, et
//! rendre un rapport parfaitement crédible sur un travail inutile.
//!
//! **Un recalcul en échec ne défait pas l'enregistrement.** Le barème reste
//! écrit, l'erreur remonte, et l'écran la montre. C'est le seul endroit de
//! l'onglet qui laisse le système à moitié appliqué — et c'est délibéré : le
//! rejeu est **idempotent**, donc le relancer suffit. Un rollback, lui, rendrait
//! barème et classement incohérents dans l'autre sens, et cette incohérence-là
//! ne se répare pas en réessayant.

use crate::app::competitions::domain::competition_rules::{CompetitionRules, RankingRules};
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::competitions::ports::IRankingRecomputePort;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;

#[derive(Debug)]
pub struct UpdateRankingSettingsCommand {
    pub season_id: SeasonId,
    pub ranking_rules: RankingRules,
}

/// Ce que l'écran annonce au retour : un décompte, pas une promesse.
#[derive(Debug, PartialEq, Eq)]
pub struct RankingSettingsOutcome {
    pub matches_replayed: u32,
    pub teams: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateRankingSettingsError {
    SeasonNotFound,
    /// **Le barème est enregistré**, seul le rejeu a échoué.
    ///
    /// La variante porte ce fait parce que l'écran doit le dire : proposer de
    /// « réessayer » n'a de sens que si l'on sait que l'enregistrement, lui, a
    /// pris.
    RecomputeFailed(String),
    Database(String),
}

impl From<SeasonRepositoryError> for UpdateRankingSettingsError {
    fn from(e: SeasonRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UpdateRankingSettingsCommand,
    season_repo: &dyn ISeasonRepository,
    recompute: &dyn IRankingRecomputePort,
) -> Result<RankingSettingsOutcome, UpdateRankingSettingsError> {
    let nom = season_repo
        .find_base_info(&cmd.season_id)
        .await?
        .ok_or(UpdateRankingSettingsError::SeasonNotFound)?
        .name;

    // **Les tiers sont relus.** `save_rules` écrit `CompetitionRules` entier, et
    // ce panneau n'édite que le barème : ne pas les relire effacerait budgets,
    // rosters et coups de pouce de tous les tiers.
    let courantes = season_repo
        .find_rules(&cmd.season_id)
        .await?
        .ok_or(UpdateRankingSettingsError::SeasonNotFound)?;

    let nouvelles = CompetitionRules {
        ranking_rules: cmd.ranking_rules,
        tiers: courantes.tiers,
    };
    season_repo
        .save_rules(&cmd.season_id, &nom, &nouvelles)
        .await?;

    let rapport = recompute
        .recompute_season(&cmd.season_id.to_string())
        .await
        .map_err(UpdateRankingSettingsError::RecomputeFailed)?;

    Ok(RankingSettingsOutcome {
        matches_replayed: rapport.matches_replayed,
        teams: rapport.teams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
    use crate::app::competitions::domain::competition_rules::{
        Activated, AggressiveBonus, DefensiveBonus, MaxTdConceded, MinCasualties, MinTd,
        OffensiveBonus, RankingPoints, TiebreakCode, TiebreakConfig,
    };
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{SeasonBaseInfo, SeasonFull};
    use crate::app::competitions::ports::RecomputeReportDto;
    use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
    use crate::app::shared_kernel::bloodbowl::tier::{CreationBudget, StartingXp, TierName};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    /// **Le journal partagé est ce qui rend l'ordre observable.** Sans lui, on
    /// ne pourrait affirmer que « les deux ont eu lieu » — jamais que l'un a
    /// précédé l'autre, qui est tout l'enjeu.
    type Journal = Arc<Mutex<Vec<&'static str>>>;

    // ── Doublures ────────────────────────────────────────────────────────────

    struct FakeSeasonRepo {
        nom: Option<String>,
        regles: Option<CompetitionRules>,
        journal: Journal,
        ecrit: Mutex<Option<(String, CompetitionRules)>>,
    }

    #[async_trait]
    impl ISeasonRepository for FakeSeasonRepo {
        async fn save(&self, _: &CompetitionSeason) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_latest_season_id(
            &self,
            _: &CompetitionId,
        ) -> Result<Option<SeasonId>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_space_id(
            &self,
            _: &SeasonId,
        ) -> Result<Option<String>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_base_info(
            &self,
            _: &SeasonId,
        ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> {
            Ok(self.nom.clone().map(|name| SeasonBaseInfo { name }))
        }
        async fn find_rules(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionRules>, SeasonRepositoryError> {
            Ok(self.regles.clone())
        }
        async fn save_rules(
            &self,
            _: &SeasonId,
            name: &str,
            rules: &CompetitionRules,
        ) -> Result<(), SeasonRepositoryError> {
            self.journal.lock().unwrap().push("save_rules");
            *self.ecrit.lock().unwrap() = Some((name.to_string(), rules.clone()));
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
        ) -> Result<Option<SeasonFull>, SeasonRepositoryError> {
            Ok(None)
        }
    }

    struct FakeRecompute {
        journal: Journal,
        echoue: bool,
    }

    #[async_trait]
    impl IRankingRecomputePort for FakeRecompute {
        async fn recompute_season(&self, _: &str) -> Result<RecomputeReportDto, String> {
            self.journal.lock().unwrap().push("recompute");
            match self.echoue {
                true => Err("le rejeu a échoué".to_string()),
                false => Ok(RecomputeReportDto {
                    matches_replayed: 24,
                    teams: 8,
                }),
            }
        }
    }

    // ── Fixtures ─────────────────────────────────────────────────────────────

    fn bareme(victoire: u32) -> RankingRules {
        RankingRules {
            win_points: RankingPoints::try_new(victoire).unwrap(),
            draw_points: RankingPoints::try_new(1).unwrap(),
            lose_points: RankingPoints::try_new(0).unwrap(),
            offensive_bonus: OffensiveBonus {
                activated: Activated(false),
                min_td: MinTd::try_new(2).unwrap(),
                points: RankingPoints::try_new(1).unwrap(),
            },
            defensive_bonus: DefensiveBonus {
                activated: Activated(false),
                points: RankingPoints::try_new(1).unwrap(),
                max_td_conceded: MaxTdConceded::try_new(0).unwrap(),
            },
            aggressive_bonus: AggressiveBonus {
                activated: Activated(false),
                points: RankingPoints::try_new(1).unwrap(),
                min_casualties: MinCasualties::try_new(3).unwrap(),
            },
            tiebreakers: TiebreakConfig::all_active(vec![TiebreakCode::try_new("nb_td").unwrap()])
                .unwrap(),
        }
    }

    fn tier(nom: &str) -> crate::app::competitions::domain::competition_rules::TierRule {
        crate::app::competitions::domain::competition_rules::TierRule {
            name: TierName::try_new(nom.to_string()).unwrap(),
            budget: CreationBudget(1000),
            starting_xp: StartingXp::try_new(0).unwrap(),
            rosters: vec!["HUMAN".into()],
            inducements: vec!["BABE".into()],
            star_players: vec![],
        }
    }

    fn decor(echoue: bool) -> (FakeSeasonRepo, FakeRecompute, Journal) {
        let journal: Journal = Arc::new(Mutex::new(vec![]));
        (
            FakeSeasonRepo {
                nom: Some("Saison 3".to_string()),
                regles: Some(CompetitionRules {
                    ranking_rules: bareme(2),
                    tiers: vec![tier("Élite"), tier("Amateurs")],
                }),
                journal: journal.clone(),
                ecrit: Mutex::new(None),
            },
            FakeRecompute {
                journal: journal.clone(),
                echoue,
            },
            journal,
        )
    }

    fn commande(victoire: u32) -> UpdateRankingSettingsCommand {
        UpdateRankingSettingsCommand {
            season_id: SeasonId::new(),
            ranking_rules: bareme(victoire),
        }
    }

    // ── Les scénarios ────────────────────────────────────────────────────────

    /// **L'enregistrement précède le recalcul.** Le rejeu lit le barème par son
    /// propre port : lancé avant l'écriture, il rejouerait avec l'**ancien**
    /// barème et rendrait un rapport parfaitement crédible sur un travail
    /// inutile.
    #[tokio::test]
    async fn le_bareme_est_enregistre_avant_d_etre_rejoue() {
        let (saison, rejeu, journal) = decor(false);

        let issue = execute(commande(3), &saison, &rejeu).await.expect("succès");

        assert_eq!(*journal.lock().unwrap(), vec!["save_rules", "recompute"]);
        assert_eq!(
            issue,
            RankingSettingsOutcome {
                matches_replayed: 24,
                teams: 8
            }
        );
    }

    /// **Les tiers sont relus.** `save_rules` écrit `CompetitionRules` entier,
    /// et ce panneau n'édite que le barème : sans relecture, budgets, rosters et
    /// coups de pouce de tous les tiers disparaîtraient.
    #[tokio::test]
    async fn changer_le_bareme_preserve_les_tiers() {
        let (saison, rejeu, _) = decor(false);

        execute(commande(3), &saison, &rejeu).await.expect("succès");

        let (nom, ecrites) = saison.ecrit.lock().unwrap().clone().expect("écriture");
        assert_eq!(nom, "Saison 3", "le nom de saison a été relu, pas inventé");
        assert_eq!(ecrites.tiers.len(), 2, "les tiers ont été effacés");
        assert_eq!(ecrites.tiers[0].inducements, vec!["BABE".to_string()]);
        // Et le barème, lui, est bien le nouveau.
        assert_eq!(ecrites.ranking_rules.win_points.into_inner(), 3);
    }

    /// **Un recalcul en échec ne défait pas l'enregistrement.** Le rejeu étant
    /// idempotent, le relancer suffit — là où un rollback rendrait barème et
    /// classement incohérents dans l'autre sens.
    #[tokio::test]
    async fn un_recalcul_en_echec_laisse_le_bareme_ecrit() {
        let (saison, rejeu, journal) = decor(true);

        let issue = execute(commande(3), &saison, &rejeu).await;

        assert!(
            matches!(issue, Err(UpdateRankingSettingsError::RecomputeFailed(_))),
            "attendu un échec de rejeu : {issue:?}"
        );
        assert_eq!(*journal.lock().unwrap(), vec!["save_rules", "recompute"]);
        let (_, ecrites) = saison.ecrit.lock().unwrap().clone().expect("écriture");
        assert_eq!(
            ecrites.ranking_rules.win_points.into_inner(),
            3,
            "le barème doit rester enregistré"
        );
    }

    #[tokio::test]
    async fn une_saison_introuvable_n_ecrit_rien() {
        let (mut saison, rejeu, journal) = decor(false);
        saison.nom = None;

        let issue = execute(commande(3), &saison, &rejeu).await;

        assert_eq!(issue, Err(UpdateRankingSettingsError::SeasonNotFound));
        assert!(journal.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn une_saison_sans_regles_n_ecrit_rien() {
        let (mut saison, rejeu, journal) = decor(false);
        saison.regles = None;

        let issue = execute(commande(3), &saison, &rejeu).await;

        assert_eq!(issue, Err(UpdateRankingSettingsError::SeasonNotFound));
        assert!(journal.lock().unwrap().is_empty());
    }
}
