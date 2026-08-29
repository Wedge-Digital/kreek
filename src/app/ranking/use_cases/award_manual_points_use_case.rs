//! Attribuer un point de classement manuel : forfait, sanction, rattrapage.
//!
//! **Aucun use case de modification ne lui répond.** Une ligne ne se corrige
//! pas : elle se supprime, et une autre la remplace avec son propre motif. Une
//! modification réécrirait l'histoire d'une décision.

use crate::app::ranking::domain::manual_points::{ManualPoints, ManualPointsReason};
use crate::app::ranking::ports::{IRankingAdminPort, IRankingCompetitionPort, IRankingRepository};
use crate::app::ranking::use_cases::manual_points::{autorise, ManualPointsError};

#[derive(Debug)]
pub struct AwardManualPointsCommand {
    pub season_id: String,
    pub competition_id: String,
    pub space_id: String,
    pub team_id: String,
    pub user_id: String,
    pub points: ManualPoints,
    /// **Optionnel** : le motif est facultatif à l'écran. `ManualPointsReason`
    /// garde son `not_empty` — c'est ici, et non dans le value object, que
    /// l'absence est permise.
    pub reason: Option<ManualPointsReason>,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: AwardManualPointsCommand,
    repo: &dyn IRankingRepository,
    admin: &dyn IRankingAdminPort,
    teams: &dyn IRankingCompetitionPort,
) -> Result<(), ManualPointsError> {
    if !autorise(admin, &cmd.user_id, &cmd.competition_id, &cmd.space_id).await {
        return Err(ManualPointsError::Forbidden);
    }

    // **L'équipe doit être inscrite.** Sans ce contrôle, une ligne s'écrirait
    // pour n'importe quel identifiant : elle n'apparaîtrait dans aucun
    // classement — celui-ci ne liste que les inscrits — et resterait invisible
    // dans la page de gestion, qui part des mêmes équipes. Le genre de donnée
    // orpheline qu'on découvre deux ans plus tard, sans savoir l'expliquer.
    let inscrite = teams
        .find_enrolled_teams(&cmd.season_id)
        .await
        .iter()
        .any(|t| t.team_id == cmd.team_id);
    if !inscrite {
        return Err(ManualPointsError::TeamNotEnrolled);
    }

    // **Aucune vérification de doublon.** Deux fois trois points à la même
    // équipe, ce sont deux décisions et deux motifs — refuser la seconde
    // obligerait à modifier la première, ce que ce module ne permet pas.
    repo.insert_manual_points(
        &cmd.season_id,
        &cmd.team_id,
        cmd.points.into_inner(),
        cmd.reason.as_ref().map(|r| r.as_ref()),
        &cmd.user_id,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::ranking_line::RankingLine;
    use crate::app::ranking::ports::{
        EnrolledTeamInfo, ManualPointRow, RankingGroupInfo, RankingLineFullRow, RankingLineRow,
        RankingRepositoryError, RankingRulesInfo,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // ── Doublures ────────────────────────────────────────────────────────────

    #[derive(Default)]
    struct FakeRepo {
        ecrit: Mutex<Vec<(String, String, i32, Option<String>, String)>>,
        supprimees: u64,
    }

    #[async_trait]
    impl IRankingRepository for FakeRepo {
        async fn find_latest_line(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<RankingLineRow>, RankingRepositoryError> {
            Ok(None)
        }
        async fn find_latest_lines_for_season(
            &self,
            _: &str,
        ) -> Result<Vec<RankingLineRow>, RankingRepositoryError> {
            Ok(Vec::new())
        }
        async fn insert_lines(&self, _: &[RankingLine]) -> Result<(), RankingRepositoryError> {
            Ok(())
        }
        async fn delete_lines_for_match(&self, _: &str) -> Result<(), RankingRepositoryError> {
            Ok(())
        }
        async fn find_all_lines_for_season(
            &self,
            _: &str,
        ) -> Result<Vec<RankingLineFullRow>, RankingRepositoryError> {
            Ok(Vec::new())
        }
        async fn replace_lines_for_season(
            &self,
            _: &str,
            _: &[RankingLine],
        ) -> Result<(), RankingRepositoryError> {
            Ok(())
        }
        async fn find_manual_totals_for_season(
            &self,
            _: &str,
        ) -> Result<HashMap<String, i32>, RankingRepositoryError> {
            Ok(HashMap::new())
        }
        async fn list_manual_points(
            &self,
            _: &str,
        ) -> Result<Vec<ManualPointRow>, RankingRepositoryError> {
            Ok(Vec::new())
        }
        async fn insert_manual_points(
            &self,
            season_id: &str,
            team_id: &str,
            points: i32,
            reason: Option<&str>,
            awarded_by: &str,
        ) -> Result<(), RankingRepositoryError> {
            self.ecrit.lock().unwrap().push((
                season_id.into(),
                team_id.into(),
                points,
                reason.map(str::to_string),
                awarded_by.into(),
            ));
            Ok(())
        }
        async fn delete_manual_points(
            &self,
            _: i64,
            _: &str,
        ) -> Result<u64, RankingRepositoryError> {
            Ok(self.supprimees)
        }
    }

    /// Chaque porte se répond séparément — c'est ce qui permet aux tests de
    /// dire **laquelle** a ouvert.
    struct FakeAdmin {
        competition: bool,
        espace: bool,
    }

    #[async_trait]
    impl IRankingAdminPort for FakeAdmin {
        async fn is_competition_admin(&self, _: &str, _: &str) -> bool {
            self.competition
        }
        async fn is_space_admin(&self, _: &str, _: &str) -> bool {
            self.espace
        }
    }

    struct FakeTeams {
        inscrites: Vec<String>,
    }

    #[async_trait]
    impl IRankingCompetitionPort for FakeTeams {
        async fn find_ranking_rules(&self, _: &str) -> Option<RankingRulesInfo> {
            None
        }
        async fn find_enrolled_teams(&self, _: &str) -> Vec<EnrolledTeamInfo> {
            self.inscrites
                .iter()
                .map(|id| EnrolledTeamInfo {
                    team_id: id.clone(),
                    team_name: "Équipe".into(),
                })
                .collect()
        }
        async fn find_groups(&self, _: &str) -> Vec<RankingGroupInfo> {
            Vec::new()
        }
    }

    fn commande(team: &str) -> AwardManualPointsCommand {
        AwardManualPointsCommand {
            season_id: "S1".into(),
            competition_id: "C1".into(),
            space_id: "E1".into(),
            team_id: team.into(),
            user_id: "U1".into(),
            points: ManualPoints::try_new(3).unwrap(),
            reason: Some(ManualPointsReason::try_new("forfait de l'adverse").unwrap()),
        }
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn un_non_admin_est_refuse() {
        let repo = FakeRepo::default();
        let admin = FakeAdmin {
            competition: false,
            espace: false,
        };
        let teams = FakeTeams {
            inscrites: vec!["T1".into()],
        };

        let r = execute(commande("T1"), &repo, &admin, &teams).await;

        assert_eq!(r, Err(ManualPointsError::Forbidden));
        assert!(
            repo.ecrit.lock().unwrap().is_empty(),
            "rien ne doit être écrit sur un refus"
        );
    }

    /// **Les deux portes, séparément.** Un test qui n'exercerait qu'un chemin
    /// laisserait passer la suppression de l'autre — le défaut que la carte 426
    /// a mis au jour dans `competitions`.
    #[tokio::test]
    async fn l_admin_de_competition_seul_suffit() {
        let repo = FakeRepo::default();
        let admin = FakeAdmin {
            competition: true,
            espace: false,
        };
        let teams = FakeTeams {
            inscrites: vec!["T1".into()],
        };

        assert!(execute(commande("T1"), &repo, &admin, &teams).await.is_ok());
    }

    #[tokio::test]
    async fn l_admin_d_espace_seul_suffit() {
        let repo = FakeRepo::default();
        let admin = FakeAdmin {
            competition: false,
            espace: true,
        };
        let teams = FakeTeams {
            inscrites: vec!["T1".into()],
        };

        assert!(execute(commande("T1"), &repo, &admin, &teams).await.is_ok());
    }

    /// Sans ce contrôle, la ligne s'écrirait pour un identifiant inconnu : elle
    /// n'apparaîtrait dans aucun classement — celui-ci ne liste que les
    /// inscrits — et resterait invisible dans la page de gestion.
    #[tokio::test]
    async fn une_equipe_non_inscrite_est_refusee() {
        let repo = FakeRepo::default();
        let admin = FakeAdmin {
            competition: true,
            espace: false,
        };
        let teams = FakeTeams {
            inscrites: vec!["T1".into()],
        };

        let r = execute(commande("T_INCONNUE"), &repo, &admin, &teams).await;

        assert_eq!(r, Err(ManualPointsError::TeamNotEnrolled));
        assert!(repo.ecrit.lock().unwrap().is_empty());
    }

    /// L'autorisation passe **avant** la vérification d'inscription : un
    /// non-admin n'apprend pas quelles équipes sont inscrites en essayant.
    #[tokio::test]
    async fn le_refus_d_autorisation_precede_celui_d_inscription() {
        let repo = FakeRepo::default();
        let admin = FakeAdmin {
            competition: false,
            espace: false,
        };
        let teams = FakeTeams { inscrites: vec![] };

        assert_eq!(
            execute(commande("T_INCONNUE"), &repo, &admin, &teams).await,
            Err(ManualPointsError::Forbidden)
        );
    }

    /// **Le cas passant qu'on croirait interdit.** Deux fois trois points à la
    /// même équipe, ce sont deux décisions et deux motifs.
    #[tokio::test]
    async fn deux_lignes_identiques_sont_acceptees() {
        let repo = FakeRepo::default();
        let admin = FakeAdmin {
            competition: true,
            espace: false,
        };
        let teams = FakeTeams {
            inscrites: vec!["T1".into()],
        };

        execute(commande("T1"), &repo, &admin, &teams)
            .await
            .unwrap();
        execute(commande("T1"), &repo, &admin, &teams)
            .await
            .unwrap();

        assert_eq!(repo.ecrit.lock().unwrap().len(), 2);
    }

    /// Le motif et l'auteur atteignent le dépôt tels quels — l'apostrophe
    /// comprise, qui a fait trébucher neuf charsets de ce projet.
    #[tokio::test]
    async fn le_motif_et_l_auteur_sont_transmis() {
        let repo = FakeRepo::default();
        let admin = FakeAdmin {
            competition: true,
            espace: false,
        };
        let teams = FakeTeams {
            inscrites: vec!["T1".into()],
        };

        execute(commande("T1"), &repo, &admin, &teams)
            .await
            .unwrap();

        let ecrit = repo.ecrit.lock().unwrap();
        assert_eq!(
            ecrit[0],
            (
                "S1".into(),
                "T1".into(),
                3,
                Some("forfait de l'adverse".to_string()),
                "U1".into()
            )
        );
    }
}
