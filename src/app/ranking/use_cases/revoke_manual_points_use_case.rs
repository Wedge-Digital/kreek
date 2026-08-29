//! Retirer un point de classement manuel.

use crate::app::ranking::ports::{IRankingAdminPort, IRankingRepository};
use crate::app::ranking::use_cases::manual_points::{autorise, ManualPointsError};

#[derive(Debug)]
pub struct RevokeManualPointsCommand {
    pub id: i64,
    pub season_id: String,
    pub competition_id: String,
    pub space_id: String,
    pub user_id: String,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RevokeManualPointsCommand,
    repo: &dyn IRankingRepository,
    admin: &dyn IRankingAdminPort,
) -> Result<(), ManualPointsError> {
    if !autorise(admin, &cmd.user_id, &cmd.competition_id, &cmd.space_id).await {
        return Err(ManualPointsError::Forbidden);
    }

    // Zéro ligne supprimée vaut `NotFound` — que l'identifiant n'existe pas ou
    // qu'il appartienne à une autre saison. La distinction n'apprendrait rien à
    // l'appelant, et la faire lui confirmerait l'existence d'une ligne qui
    // n'est pas la sienne.
    match repo.delete_manual_points(cmd.id, &cmd.season_id).await? {
        0 => Err(ManualPointsError::NotFound),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::ranking_line::RankingLine;
    use crate::app::ranking::ports::{
        ManualPointRow, RankingLineFullRow, RankingLineRow, RankingRepositoryError,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeRepo {
        supprimees: u64,
        appels: Mutex<Vec<(i64, String)>>,
    }

    impl FakeRepo {
        fn qui_supprime(n: u64) -> Self {
            Self {
                supprimees: n,
                appels: Mutex::new(Vec::new()),
            }
        }
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
            _: &str,
            _: &str,
            _: i32,
            _: Option<&str>,
            _: &str,
        ) -> Result<(), RankingRepositoryError> {
            Ok(())
        }
        async fn delete_manual_points(
            &self,
            id: i64,
            season_id: &str,
        ) -> Result<u64, RankingRepositoryError> {
            self.appels.lock().unwrap().push((id, season_id.into()));
            Ok(self.supprimees)
        }
    }

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

    fn commande() -> RevokeManualPointsCommand {
        RevokeManualPointsCommand {
            id: 42,
            season_id: "S1".into(),
            competition_id: "C1".into(),
            space_id: "E1".into(),
            user_id: "U1".into(),
        }
    }

    #[tokio::test]
    async fn un_non_admin_est_refuse() {
        let repo = FakeRepo::qui_supprime(1);
        let admin = FakeAdmin {
            competition: false,
            espace: false,
        };

        assert_eq!(
            execute(commande(), &repo, &admin).await,
            Err(ManualPointsError::Forbidden)
        );
        assert!(
            repo.appels.lock().unwrap().is_empty(),
            "le dépôt ne doit pas être touché sur un refus"
        );
    }

    #[tokio::test]
    async fn l_admin_de_competition_seul_suffit() {
        let repo = FakeRepo::qui_supprime(1);
        let admin = FakeAdmin {
            competition: true,
            espace: false,
        };

        assert!(execute(commande(), &repo, &admin).await.is_ok());
    }

    #[tokio::test]
    async fn l_admin_d_espace_seul_suffit() {
        let repo = FakeRepo::qui_supprime(1);
        let admin = FakeAdmin {
            competition: false,
            espace: true,
        };

        assert!(execute(commande(), &repo, &admin).await.is_ok());
    }

    /// Zéro ligne supprimée vaut `NotFound` — que l'identifiant n'existe pas ou
    /// qu'il appartienne à une autre saison. Distinguer les deux confirmerait à
    /// l'appelant l'existence d'une ligne qui n'est pas la sienne.
    #[tokio::test]
    async fn zero_ligne_supprimee_vaut_introuvable() {
        let repo = FakeRepo::qui_supprime(0);
        let admin = FakeAdmin {
            competition: true,
            espace: false,
        };

        assert_eq!(
            execute(commande(), &repo, &admin).await,
            Err(ManualPointsError::NotFound)
        );
    }

    /// **La saison est bien transmise au dépôt.** C'est elle qui referme le
    /// trou de `{point_id}`, que `space_scope` ne résout pas : la passer est la
    /// moitié applicative d'un contrôle dont l'autre moitié est le `WHERE`.
    #[tokio::test]
    async fn la_saison_accompagne_l_identifiant_jusqu_au_depot() {
        let repo = FakeRepo::qui_supprime(1);
        let admin = FakeAdmin {
            competition: true,
            espace: false,
        };

        execute(commande(), &repo, &admin).await.unwrap();

        assert_eq!(repo.appels.lock().unwrap()[0], (42, "S1".to_string()));
    }
}
