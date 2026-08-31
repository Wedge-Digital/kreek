//! Modifier les coups de pouce et les star players autorisés par tier.
//!
//! **Rien d'autre** : ni le nom, ni le budget, ni l'XP de départ, ni les
//! rosters. `TierRule` est pourtant un tout et les transporte quand même — c'est
//! `CompetitionRules::with_inducements_from` (carte 417) qui refuse tout écart.
//!
//! Ce use case n'en juge rien : il relit, appelle le domaine, convertit son
//! erreur. Rejuger ici dédoublerait la règle et la ferait diverger.

use crate::app::competitions::domain::competition_rules::{CompetitionRules, TierRule};
use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;

#[derive(Debug)]
pub struct UpdateTiersSettingsCommand {
    pub season_id: SeasonId,
    pub tiers: Vec<TierRule>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateTiersSettingsError {
    SeasonNotFound,
    /// Un champ non éditable a bougé, ou le nombre de tiers a changé.
    ///
    /// **Un refus, pas une correction.** Accepter la valeur reçue rendrait
    /// modifiable par requête forgée ce que l'écran n'ouvre pas ; la corriger en
    /// silence ferait croire à un enregistrement qui n'a pas eu lieu.
    Rejected(DomainError),
    Database(String),
}

impl From<SeasonRepositoryError> for UpdateTiersSettingsError {
    fn from(e: SeasonRepositoryError) -> Self {
        match e {
            SeasonRepositoryError::SeasonNotFound => Self::SeasonNotFound,
            autre => Self::Database(autre.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UpdateTiersSettingsCommand,
    season_repo: &dyn ISeasonRepository,
) -> Result<(), UpdateTiersSettingsError> {
    let nom = season_repo
        .find_base_info(&cmd.season_id)
        .await?
        .ok_or(UpdateTiersSettingsError::SeasonNotFound)?
        .name;

    // **Le barème est relu.** `save_rules` écrit `CompetitionRules` entier, et ce
    // panneau n'édite que les tiers : sans relecture, points de victoire, bonus
    // et critères de départage disparaîtraient.
    let courantes = season_repo
        .find_rules(&cmd.season_id)
        .await?
        .ok_or(UpdateTiersSettingsError::SeasonNotFound)?;

    let nouvelles: CompetitionRules = courantes
        .with_inducements_from(cmd.tiers)
        .map_err(UpdateTiersSettingsError::Rejected)?;

    season_repo
        .save_rules_keep_status(&cmd.season_id, &nom, &nouvelles)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
    use crate::app::competitions::domain::competition_rules::{
        Activated, AggressiveBonus, DefensiveBonus, MaxTdConceded, MinCasualties, MinTd,
        OffensiveBonus, RankingPoints, RankingRules, TiebreakCode, TiebreakConfig,
    };
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{SeasonBaseInfo, SeasonFull};
    use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
    use crate::app::shared_kernel::bloodbowl::tier::{CreationBudget, StartingXp, TierName};
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct FakeSeasonRepo {
        nom: Option<String>,
        regles: Option<CompetitionRules>,
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
        async fn save_rules_keep_status(
            &self,
            _: &SeasonId,
            name: &str,
            rules: &CompetitionRules,
        ) -> Result<(), SeasonRepositoryError> {
            *self.ecrit.lock().unwrap() = Some((name.to_string(), rules.clone()));
            Ok(())
        }
        /// **Le chemin interdit.** `save_rules` pose
        /// `status = 'rules_selected'` et ferait régresser une saison en cours
        /// sous `ready` (carte 485). Le faux refuse plutôt que d'enregistrer :
        /// chaque test de ce use case devient ainsi un garde-fou, sans qu'aucun
        /// n'ait à y penser.
        async fn save_rules(
            &self,
            _: &SeasonId,
            _: &str,
            _: &CompetitionRules,
        ) -> Result<(), SeasonRepositoryError> {
            unreachable!(
                "un panneau de réglages doit appeler save_rules_keep_status : \
                 save_rules écrase le statut de la saison (carte 485)"
            )
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

    // ── Fixtures ─────────────────────────────────────────────────────────────

    const POINTS_VICTOIRE: u32 = 3;

    fn tier(nom: &str, coups: &[&str], stars: &[&str]) -> TierRule {
        TierRule {
            name: TierName::try_new(nom.to_string()).unwrap(),
            budget: CreationBudget(1000),
            starting_xp: StartingXp::try_new(6).unwrap(),
            rosters: vec!["HUMAN".into()],
            inducements: coups.iter().map(|c| c.to_string()).collect(),
            star_players: stars.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Le barème porte des **points de victoire distincts du défaut**,
    /// précisément pour qu'un test qui le perdrait s'en aperçoive.
    fn regles(tiers: Vec<TierRule>) -> CompetitionRules {
        CompetitionRules {
            ranking_rules: RankingRules {
                win_points: RankingPoints::try_new(POINTS_VICTOIRE).unwrap(),
                draw_points: RankingPoints::try_new(1).unwrap(),
                lose_points: RankingPoints::try_new(0).unwrap(),
                offensive_bonus: OffensiveBonus {
                    activated: Activated(true),
                    min_td: MinTd::try_new(2).unwrap(),
                    points: RankingPoints::try_new(1).unwrap(),
                },
                defensive_bonus: DefensiveBonus {
                    activated: Activated(false),
                    points: RankingPoints::try_new(1).unwrap(),
                    max_td_conceded: MaxTdConceded::try_new(1).unwrap(),
                },
                aggressive_bonus: AggressiveBonus {
                    activated: Activated(false),
                    points: RankingPoints::try_new(1).unwrap(),
                    min_casualties: MinCasualties::try_new(3).unwrap(),
                },
                tiebreakers: TiebreakConfig::all_active(vec![
                    TiebreakCode::try_new("nb_td").unwrap()
                ])
                .unwrap(),
            },
            tiers,
        }
    }

    fn depot(tiers: Vec<TierRule>) -> FakeSeasonRepo {
        FakeSeasonRepo {
            nom: Some("Saison 4".to_string()),
            regles: Some(regles(tiers)),
            ecrit: Mutex::new(None),
        }
    }

    fn commande(tiers: Vec<TierRule>) -> UpdateTiersSettingsCommand {
        UpdateTiersSettingsCommand {
            season_id: SeasonId::new(),
            tiers,
        }
    }

    // ── Les scénarios ────────────────────────────────────────────────────────

    /// **Le barème est relu.** `save_rules` écrit `CompetitionRules` entier et ce
    /// panneau n'édite que les tiers : sans relecture, points, bonus et critères
    /// de départage disparaîtraient — silencieusement.
    /// **Le chemin qui réécrit le statut n'est jamais emprunté** (carte 485).
    ///
    /// `save_rules` pose `status = 'rules_selected'`. L'emprunter ici ferait
    /// régresser une saison en cours sous `ready` : la carte de la compétition
    /// mènerait à l'étape 2 du magicien, et la création d'équipe serait
    /// refusée. Le défaut serait invisible — l'enregistrement réussit.
    ///
    /// Le faux **refuse** cette méthode au lieu de la journaliser : chaque test
    /// de ce use case garde donc l'invariant, sans qu'aucun n'ait à y penser.
    /// Ce test-ci existe pour que l'intention porte un nom.
    #[tokio::test]
    async fn le_chemin_qui_reecrit_le_statut_n_est_jamais_emprunte() {
        let depot = depot(vec![tier("Élite", &["BABE"], &[])]);

        execute(commande(vec![tier("Élite", &["BABE"], &[])]), &depot)
            .await
            .expect("le faux aurait paniqué sur save_rules");

        assert!(
            depot.ecrit.lock().unwrap().is_some(),
            "l'écriture a bien eu lieu"
        );
    }

    #[tokio::test]
    async fn changer_les_coups_de_pouce_preserve_le_bareme() {
        let depot = depot(vec![tier("Élite", &["BABE"], &[])]);

        execute(
            commande(vec![tier("Élite", &["BABE", "BLOODWEISER"], &["GRIFF"])]),
            &depot,
        )
        .await
        .expect("cas nominal");

        let (nom, ecrites) = depot.ecrit.lock().unwrap().clone().expect("écriture");
        assert_eq!(nom, "Saison 4", "le nom de saison a été relu, pas inventé");
        assert_eq!(
            ecrites.ranking_rules.win_points.into_inner(),
            POINTS_VICTOIRE,
            "le barème a été écrasé"
        );
        assert!(ecrites.ranking_rules.offensive_bonus.activated.0);
        assert_eq!(ecrites.tiers[0].inducements.len(), 2);
        assert_eq!(ecrites.tiers[0].star_players, vec!["GRIFF".to_string()]);
    }

    /// Un tier **sans aucun coup de pouce** est valide : aucune borne basse.
    #[tokio::test]
    async fn un_tier_sans_coup_de_pouce_est_accepte() {
        let depot = depot(vec![tier("Élite", &["BABE"], &["GRIFF"])]);

        execute(commande(vec![tier("Élite", &[], &[])]), &depot)
            .await
            .expect("liste vide valide");

        let (_, ecrites) = depot.ecrit.lock().unwrap().clone().expect("écriture");
        assert!(ecrites.tiers[0].inducements.is_empty());
        assert!(ecrites.tiers[0].star_players.is_empty());
    }

    /// **Le refus vient du domaine**, et le use case ne fait que le relayer.
    /// Rejuger ici dédoublerait la règle et la ferait diverger.
    #[tokio::test]
    async fn un_budget_modifie_est_refuse_et_rien_n_est_ecrit() {
        let depot = depot(vec![tier("Élite", &["BABE"], &[])]);
        let mut forge = tier("Élite", &["BABE"], &[]);
        forge.budget = CreationBudget(999_999);

        let issue = execute(commande(vec![forge]), &depot).await;

        match issue {
            Err(UpdateTiersSettingsError::Rejected(DomainError::ImmutableTierField {
                field,
                ..
            })) => assert_eq!(field, "budget"),
            autre => panic!("attendu un refus de budget : {autre:?}"),
        }
        assert!(
            depot.ecrit.lock().unwrap().is_none(),
            "un refus ne doit rien écrire"
        );
    }

    #[tokio::test]
    async fn un_tier_ajoute_est_refuse() {
        let depot = depot(vec![tier("Élite", &[], &[])]);

        let issue = execute(
            commande(vec![tier("Élite", &[], &[]), tier("Amateurs", &[], &[])]),
            &depot,
        )
        .await;

        assert!(matches!(
            issue,
            Err(UpdateTiersSettingsError::Rejected(
                DomainError::TierCountChanged { .. }
            ))
        ));
    }

    #[tokio::test]
    async fn une_saison_sans_regles_est_refusee() {
        let depot = FakeSeasonRepo {
            nom: Some("Saison 4".to_string()),
            regles: None,
            ecrit: Mutex::new(None),
        };

        let issue = execute(commande(vec![]), &depot).await;

        assert_eq!(issue, Err(UpdateTiersSettingsError::SeasonNotFound));
    }
}
