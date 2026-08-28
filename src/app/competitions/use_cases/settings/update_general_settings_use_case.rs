//! Renommer une compétition, renommer sa saison, changer son logo.
//!
//! # Les deux relectures sont le cœur de ce use case
//!
//! Le panneau n'édite ni les administrateurs ni le barème. Mais les deux
//! écritures qu'il déclenche les portent :
//!
//! - `update_base_info` prend `admin_ids` ;
//! - `save_rules` prend les `rules`.
//!
//! Ne pas les relire **viderait les administrateurs et effacerait tout le
//! barème** — silencieusement, l'écran renvoyant ensuite un panneau
//! parfaitement normal. C'est pourquoi les deux lectures ne sont pas une
//! commodité mais la raison d'être de cette fonction.
//!
//! # Deux écritures, une seule intention
//!
//! Le nom de compétition vit dans `competitions`, celui de saison dans
//! `competition_seasons`. Le `CLAUDE.md` interdisant au handler d'appeler deux
//! use cases, c'est ici que l'intention se recompose.
//!
//! **Pas de transaction commune** : deux libellés qu'aucun invariant ne lie.
//! Un nom de compétition changé sans le nom de saison n'est pas un état
//! incohérent, seulement un travail à moitié fait — et l'écran renvoie l'état
//! réel au retour.

use crate::app::competitions::domain::competition_repository_port::{
    CompetitionRepositoryError, ICompetitionRepository,
};
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::bloodbowl::season_name::SeasonName;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};

#[derive(Debug)]
pub struct UpdateGeneralSettingsCommand {
    pub competition_id: CompetitionId,
    pub space_id: SpaceId,
    pub season_id: SeasonId,
    pub name: CompetitionName,
    pub season_name: SeasonName,
    pub logo: CloudinaryImage,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateGeneralSettingsError {
    CompetitionNotFound,
    SeasonNotFound,
    NameAlreadyTaken,
    /// Un identifiant d'administrateur illisible en base.
    ///
    /// **Un refus, pas un filtrage.** Écarter l'identifiant fautif — le réflexe
    /// naturel, un `filter_map` — retirerait cet administrateur de la
    /// compétition au premier renommage venu : exactement la perte que la
    /// relecture existe pour empêcher, et sans un mot. Mieux vaut refuser le
    /// renommage et laisser voir la ligne corrompue.
    MalformedAdminId(String),
    Database(String),
}

impl From<CompetitionRepositoryError> for UpdateGeneralSettingsError {
    fn from(e: CompetitionRepositoryError) -> Self {
        match e {
            CompetitionRepositoryError::CompetitionNameAlreadyTaken => Self::NameAlreadyTaken,
            CompetitionRepositoryError::CompetitionNotFound => Self::CompetitionNotFound,
            CompetitionRepositoryError::Database(msg) => Self::Database(msg),
        }
    }
}

impl From<SeasonRepositoryError> for UpdateGeneralSettingsError {
    fn from(e: SeasonRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UpdateGeneralSettingsCommand,
    competition_repo: &dyn ICompetitionRepository,
    season_repo: &dyn ISeasonRepository,
) -> Result<(), UpdateGeneralSettingsError> {
    let courante = competition_repo
        .find_base_info(&cmd.competition_id)
        .await?
        .ok_or(UpdateGeneralSettingsError::CompetitionNotFound)?;

    // Le contrôle d'unicité n'a lieu que si le nom **change** : sans cette
    // garde, réenregistrer le panneau sans toucher au nom se heurterait à la
    // compétition elle-même.
    if courante.name != cmd.name.value()
        && competition_repo
            .name_exists_in_space(&cmd.name, &cmd.space_id)
            .await?
    {
        return Err(UpdateGeneralSettingsError::NameAlreadyTaken);
    }

    let admin_ids = decoder_les_administrateurs(&courante.admin_ids)?;
    competition_repo
        .update_base_info(&cmd.competition_id, &cmd.name, &cmd.logo, &admin_ids)
        .await?;

    let regles = season_repo
        .find_rules(&cmd.season_id)
        .await?
        .ok_or(UpdateGeneralSettingsError::SeasonNotFound)?;
    season_repo
        .save_rules(&cmd.season_id, cmd.season_name.as_ref(), &regles)
        .await?;

    Ok(())
}

/// `find_base_info` rend des `String`, `update_base_info` exige des `CoachId` :
/// la relecture ne suffit pas, il faut convertir.
///
/// Et la conversion peut échouer — d'où un `Result` plutôt qu'un `filter_map`,
/// cf. `MalformedAdminId`.
fn decoder_les_administrateurs(
    bruts: &[String],
) -> Result<Vec<CoachId>, UpdateGeneralSettingsError> {
    bruts
        .iter()
        .map(|brut| {
            CoachId::try_new(brut)
                .map_err(|_| UpdateGeneralSettingsError::MalformedAdminId(brut.clone()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition::Competition;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
    use crate::app::competitions::domain::competition_repository_port::{
        CompetitionBaseInfo, CompetitionSummary, CompetitionWithSeasons,
    };
    use crate::app::competitions::domain::competition_rules::{
        Activated, AggressiveBonus, CompetitionRules, DefensiveBonus, MaxTdConceded, MinCasualties,
        MinTd, OffensiveBonus, RankingPoints, RankingRules, TiebreakCode, TiebreakConfig,
    };
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{SeasonBaseInfo, SeasonFull};
    use async_trait::async_trait;
    use std::sync::Mutex;

    // ── Doublures qui enregistrent ───────────────────────────────────────────
    //
    // Des doublures **muettes** ne prouveraient rien ici : tout l'enjeu est de
    // constater ce qui a été écrit. Une relecture oubliée passe la compilation,
    // rend `Ok(())`, et ne se voit que dans les arguments reçus.

    #[derive(Default)]
    struct FakeCompetitionRepo {
        nom_courant: String,
        admins: Vec<String>,
        nom_deja_pris: bool,
        /// Les `admin_ids` reçus par `update_base_info`, et le nom écrit.
        ecrit: Mutex<Option<(String, Vec<CoachId>)>>,
        /// Compte les appels au contrôle d'unicité — pour prouver qu'un nom
        /// inchangé ne le déclenche pas.
        controles_unicite: Mutex<u32>,
    }

    #[async_trait]
    impl ICompetitionRepository for FakeCompetitionRepo {
        async fn find_space_id(
            &self,
            _: &CompetitionId,
        ) -> Result<Option<String>, CompetitionRepositoryError> {
            Ok(None)
        }
        async fn name_exists_in_space(
            &self,
            _: &CompetitionName,
            _: &SpaceId,
        ) -> Result<bool, CompetitionRepositoryError> {
            *self.controles_unicite.lock().unwrap() += 1;
            Ok(self.nom_deja_pris)
        }
        async fn save(&self, _: &Competition) -> Result<(), CompetitionRepositoryError> {
            Ok(())
        }
        async fn find_by_space_id(
            &self,
            _: &SpaceId,
        ) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> {
            Ok(vec![])
        }
        async fn find_with_seasons(
            &self,
            _: &SpaceId,
        ) -> Result<Vec<CompetitionWithSeasons>, CompetitionRepositoryError> {
            Ok(vec![])
        }
        async fn find_base_info(
            &self,
            _: &CompetitionId,
        ) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> {
            Ok(Some(CompetitionBaseInfo {
                name: self.nom_courant.clone(),
                logo: None,
                admin_ids: self.admins.clone(),
                admin_names: vec![],
            }))
        }
        async fn update_base_info(
            &self,
            _: &CompetitionId,
            name: &CompetitionName,
            _: &CloudinaryImage,
            admin_ids: &[CoachId],
        ) -> Result<(), CompetitionRepositoryError> {
            *self.ecrit.lock().unwrap() = Some((name.value().to_string(), admin_ids.to_vec()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeSeasonRepo {
        regles: Option<CompetitionRules>,
        /// Le nom et les règles reçus par `save_rules`.
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
            Ok(None)
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

    fn bareme(victoire: u32) -> CompetitionRules {
        CompetitionRules {
            ranking_rules: RankingRules {
                win_points: RankingPoints::try_new(victoire).unwrap(),
                draw_points: RankingPoints::try_new(1).unwrap(),
                lose_points: RankingPoints::try_new(0).unwrap(),
                offensive_bonus: OffensiveBonus {
                    activated: Activated(false),
                    min_td: MinTd::try_new(2).unwrap(),
                    points: RankingPoints::try_new(1).unwrap(),
                },
                // Les deux bonus sont construits littéralement plutôt que par
                // leurs `default_*`, qui sont privés au module — les rendre
                // publics pour un test changerait la production.
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
                tiebreakers: TiebreakConfig::all_active(vec![
                    TiebreakCode::try_new("nb_td").unwrap()
                ])
                .unwrap(),
            },
            tiers: vec![],
        }
    }

    fn commande(nom: &str) -> UpdateGeneralSettingsCommand {
        UpdateGeneralSettingsCommand {
            competition_id: CompetitionId::new(),
            space_id: SpaceId::new(),
            season_id: SeasonId::new(),
            name: CompetitionName::try_new(nom.to_string()).unwrap(),
            season_name: SeasonName::try_new("Saison 2".to_string()).unwrap(),
            logo: CloudinaryImage::try_new(
                "https://res.cloudinary.com/demo/image/upload/v1/x.jpg".to_string(),
            )
            .unwrap(),
        }
    }

    fn depots(nom_courant: &str, admins: Vec<&str>) -> (FakeCompetitionRepo, FakeSeasonRepo) {
        (
            FakeCompetitionRepo {
                nom_courant: nom_courant.to_string(),
                admins: admins.into_iter().map(|a| a.to_string()).collect(),
                ..Default::default()
            },
            FakeSeasonRepo {
                regles: Some(bareme(3)),
                ..Default::default()
            },
        )
    }

    // ── Les deux relectures — le cœur de la carte ────────────────────────────

    /// **Sans cette relecture, un renommage viderait les administrateurs.**
    /// `update_base_info` les prend en argument, et le panneau ne les édite pas.
    #[tokio::test]
    async fn le_renommage_preserve_les_administrateurs() {
        let admin = CoachId::new().to_string();
        let autre = CoachId::new().to_string();
        let (comp, saison) = depots("Ancien nom", vec![&admin, &autre]);

        execute(commande("Nouveau nom"), &comp, &saison)
            .await
            .expect("renommage");

        let (nom, admins) = comp.ecrit.lock().unwrap().clone().expect("écriture");
        assert_eq!(nom, "Nouveau nom");
        let ecrits: Vec<String> = admins.iter().map(|a| a.to_string()).collect();
        assert_eq!(
            ecrits,
            vec![admin, autre],
            "les administrateurs ont été perdus"
        );
    }

    /// **Sans cette relecture, un renommage effacerait tout le barème.**
    /// `save_rules` les prend en argument, et le panneau ne les édite pas.
    #[tokio::test]
    async fn le_renommage_preserve_le_bareme() {
        let (comp, saison) = depots("Ancien nom", vec![]);

        execute(commande("Nouveau nom"), &comp, &saison)
            .await
            .expect("renommage");

        let (nom, regles) = saison.ecrit.lock().unwrap().clone().expect("écriture");
        assert_eq!(nom, "Saison 2");
        assert_eq!(
            regles.ranking_rules.win_points.into_inner(),
            3,
            "le barème a été écrasé"
        );
    }

    // ── Le nom ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn un_nom_deja_pris_est_refuse() {
        let (mut comp, saison) = depots("Ancien nom", vec![]);
        comp.nom_deja_pris = true;

        let issue = execute(commande("Nom occupé"), &comp, &saison).await;

        assert_eq!(issue, Err(UpdateGeneralSettingsError::NameAlreadyTaken));
        assert!(
            comp.ecrit.lock().unwrap().is_none(),
            "un refus ne doit rien écrire"
        );
    }

    /// **Un nom inchangé ne déclenche pas le contrôle d'unicité.** Sans cette
    /// garde, réenregistrer le panneau sans toucher au nom se heurterait à la
    /// compétition elle-même — elle porte déjà ce nom.
    #[tokio::test]
    async fn un_nom_inchange_ne_declenche_pas_le_controle_d_unicite() {
        let (mut comp, saison) = depots("Même nom", vec![]);
        comp.nom_deja_pris = true; // s'il était consulté, il refuserait

        execute(commande("Même nom"), &comp, &saison)
            .await
            .expect("un nom inchangé passe");

        assert_eq!(*comp.controles_unicite.lock().unwrap(), 0);
    }

    // ── Les identifiants illisibles ──────────────────────────────────────────

    /// **Un refus, pas un filtrage.** Écarter l'identifiant fautif retirerait
    /// cet administrateur au premier renommage venu — la perte même que la
    /// relecture existe pour empêcher, et sans un mot.
    #[tokio::test]
    async fn un_identifiant_d_administrateur_illisible_refuse_le_renommage() {
        let bon = CoachId::new().to_string();
        let (comp, saison) = depots("Ancien nom", vec![&bon, "pas-un-ulid"]);

        let issue = execute(commande("Nouveau nom"), &comp, &saison).await;

        assert_eq!(
            issue,
            Err(UpdateGeneralSettingsError::MalformedAdminId(
                "pas-un-ulid".to_string()
            ))
        );
        assert!(
            comp.ecrit.lock().unwrap().is_none(),
            "rien ne doit être écrit sur une ligne corrompue"
        );
    }

    // ── Les absences ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn une_saison_sans_regles_est_refusee() {
        let (comp, mut saison) = depots("Ancien nom", vec![]);
        saison.regles = None;

        let issue = execute(commande("Nouveau nom"), &comp, &saison).await;

        assert_eq!(issue, Err(UpdateGeneralSettingsError::SeasonNotFound));
    }
}
