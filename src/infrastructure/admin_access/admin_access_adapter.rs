//! L'adapter unique du service `est_admin` (carte 570).
//!
//! Il est le seul endroit qui sache où vivent les admins : le profil d'espace
//! dans `spaces`, les admins de compétition dans `competitions`. Il vit dans
//! l'infrastructure pour cette raison, et tous les BCs l'interrogent par le port
//! du noyau partagé.
//!
//! Les deux corps sont repris de `infrastructure/teams/access_adapter.rs`, qu'il
//! remplace — moins la comparaison par nom, plus l'exception exploitant.

use crate::app::auth::ports::IUserRepository;
use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::shared_kernel::bloodbowl::admin_access::IAdminAccessPort;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Le compte qui administre tous les espaces et toutes les compétitions,
/// quel que soit son profil — reconnu par son nom, résolu une fois en
/// identifiant.
///
/// Codé en dur, et dans l'infrastructure : c'est une notion d'exploitation de
/// kreek, pas une règle d'un BC. Il vivait dans `web/app_menu.rs`, qui n'en
/// faisait qu'une entrée de menu ; il donne désormais **les droits** (décision
/// du 2026-10-03). Si la valeur devait varier d'un environnement à l'autre,
/// elle passerait en configuration — pas avant.
pub const COMPTE_EXPLOITANT: &str = "Bagouze";

pub struct AdminAccessAdapter {
    space_repo: Arc<dyn ISpaceRepository>,
    competition_repo: Arc<dyn ICompetitionRepository>,
    user_repo: Arc<dyn IUserRepository>,
    /// L'identifiant du compte exploitant, résolu à la première question.
    ///
    /// **Une absence n'est pas retenue** : tant que le compte n'existe pas —
    /// une base neuve, un environnement de test —, la question se repose. Le
    /// compte créé ensuite est reconnu sans redémarrage.
    exploitant: OnceCell<String>,
}

impl AdminAccessAdapter {
    pub fn new(
        space_repo: Arc<dyn ISpaceRepository>,
        competition_repo: Arc<dyn ICompetitionRepository>,
        user_repo: Arc<dyn IUserRepository>,
    ) -> Self {
        Self {
            space_repo,
            competition_repo,
            user_repo,
            exploitant: OnceCell::new(),
        }
    }

    async fn est_l_exploitant(&self, user_id: &CoachId) -> bool {
        let resolu = self
            .exploitant
            .get_or_try_init(|| async {
                match self.user_repo.find_by_coach_name(COMPTE_EXPLOITANT).await {
                    Ok(Some(user)) => Ok(user.id.to_string()),
                    _ => Err(()),
                }
            })
            .await;
        matches!(resolu, Ok(id) if *id == user_id.to_string())
    }
}

#[async_trait]
impl IAdminAccessPort for AdminAccessAdapter {
    async fn is_space_admin(&self, user_id: &CoachId, space_id: &SpaceId) -> bool {
        if self.est_l_exploitant(user_id).await {
            return true;
        }
        matches!(
            self.space_repo
                .find_member_profile(user_id, space_id)
                .await
                .ok()
                .flatten(),
            Some(SpaceProfile::SpaceAdmin)
        )
    }

    /// **Par identifiant seul** (décision du 2026-10-03) : les admins ne
    /// s'enregistrent que par identifiant, et `admin_names` n'est que leur
    /// pseudonyme obtenu par jointure. Le comparer au nom du visiteur n'apportait
    /// rien dans une base cohérente.
    ///
    /// Un dépôt en échec rend `false` : un contrôle d'accès échoue fermé.
    async fn is_competition_admin(
        &self,
        user_id: &CoachId,
        competition_id: &CompetitionId,
    ) -> bool {
        if self.est_l_exploitant(user_id).await {
            return true;
        }
        let coach_id = user_id.to_string();
        match self.competition_repo.find_base_info(competition_id).await {
            Ok(Some(info)) => info.admin_ids.iter().any(|x| *x == coach_id),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::io::repository::user_repository::UserRepository;
    use crate::app::competitions::io::repository::competition_repository::CompetitionRepository;
    use crate::app::shared_kernel::identity::ids::{CoachId, EntityId};
    use crate::app::spaces::io::repository::space_repository::SpaceRepository;
    use sqlx::PgPool;

    fn adapter(pool: &PgPool) -> AdminAccessAdapter {
        AdminAccessAdapter::new(
            Arc::new(SpaceRepository::new(pool.clone())),
            Arc::new(CompetitionRepository::new(pool.clone())),
            Arc::new(UserRepository::new(pool.clone())),
        )
    }

    async fn compte(pool: &PgPool, id: &CoachId, nom: &str) {
        sqlx::query(
            "INSERT INTO auth__users (id, coach_name, email, password_hash) VALUES ($1, $2, $3, 'x')",
        )
        .bind(id.to_string())
        .bind(nom)
        .bind(format!("{nom}@kreek.test"))
        .execute(pool)
        .await
        .unwrap();
    }

    async fn membre(pool: &PgPool, espace: &SpaceId, coach: &CoachId, profil: &str) {
        sqlx::query(
            "INSERT INTO spaces__user_space (space_id, coach_id, profile) VALUES ($1, $2, $3)",
        )
        .bind(espace.to_string())
        .bind(coach.to_string())
        .bind(profil)
        .execute(pool)
        .await
        .unwrap();
    }

    /// Une compétition dont `admin` est l'admin, sous le pseudonyme `nom`.
    async fn competition(
        pool: &PgPool,
        espace: &SpaceId,
        admin: &CoachId,
        nom: &str,
    ) -> CompetitionId {
        let competition: CompetitionId = EntityId::new();
        sqlx::query(
            "INSERT INTO competitions (id, space_id, name, logo) VALUES ($1, $2, 'Ligue', '')",
        )
        .bind(competition.to_string())
        .bind(espace.to_string())
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO competitions_members (competition_id, coach_id, competition_profile)
             VALUES ($1, $2, 'CompetitionAdmin')",
        )
        .bind(competition.to_string())
        .bind(admin.to_string())
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO spaces__user_cache (id, coach_name, email) VALUES ($1, $2, $3)")
            .bind(admin.to_string())
            .bind(nom)
            .bind(format!("{nom}@kreek.test"))
            .execute(pool)
            .await
            .unwrap();
        competition
    }

    #[sqlx::test]
    async fn l_admin_d_espace_l_est_et_le_simple_membre_non(pool: PgPool) {
        let espace: SpaceId = EntityId::new();
        let (admin, simple): (CoachId, CoachId) = (EntityId::new(), EntityId::new());
        membre(&pool, &espace, &admin, "SpaceAdmin").await;
        membre(&pool, &espace, &simple, "SpaceUser").await;
        let a = adapter(&pool);
        assert!(a.is_space_admin(&admin, &espace).await);
        assert!(!a.is_space_admin(&simple, &espace).await);
    }

    #[sqlx::test]
    async fn l_admin_de_competition_est_reconnu_par_son_identifiant(pool: PgPool) {
        let espace: SpaceId = EntityId::new();
        let admin: CoachId = EntityId::new();
        let competition = competition(&pool, &espace, &admin, "Grumbak").await;
        assert!(
            adapter(&pool)
                .is_competition_admin(&admin, &competition)
                .await
        );
    }

    /// Décision du 2026-10-03 : le nom seul ne suffit plus. Un autre compte qui
    /// porterait le pseudonyme de l'admin — cache désynchronisé, base importée —
    /// n'hérite pas de ses droits.
    #[sqlx::test]
    async fn le_pseudonyme_de_l_admin_ne_suffit_pas(pool: PgPool) {
        let espace: SpaceId = EntityId::new();
        let (admin, homonyme): (CoachId, CoachId) = (EntityId::new(), EntityId::new());
        let competition = competition(&pool, &espace, &admin, "Grumbak").await;
        assert!(
            !adapter(&pool)
                .is_competition_admin(&homonyme, &competition)
                .await
        );
    }

    /// Le compte exploitant a tous les droits, sans être membre de rien.
    #[sqlx::test]
    async fn le_compte_exploitant_est_admin_partout(pool: PgPool) {
        let espace: SpaceId = EntityId::new();
        let (exploitant, admin): (CoachId, CoachId) = (EntityId::new(), EntityId::new());
        compte(&pool, &exploitant, COMPTE_EXPLOITANT).await;
        let competition = competition(&pool, &espace, &admin, "Grumbak").await;
        let a = adapter(&pool);
        assert!(a.is_space_admin(&exploitant, &espace).await);
        assert!(a.is_competition_admin(&exploitant, &competition).await);
    }

    /// Une base sans compte exploitant : l'absence n'est pas retenue, et le
    /// compte créé ensuite est reconnu par le même adapter.
    #[sqlx::test]
    async fn le_compte_exploitant_cree_apres_coup_est_reconnu(pool: PgPool) {
        let espace: SpaceId = EntityId::new();
        let exploitant: CoachId = EntityId::new();
        let a = adapter(&pool);
        assert!(!a.is_space_admin(&exploitant, &espace).await);
        compte(&pool, &exploitant, COMPTE_EXPLOITANT).await;
        assert!(a.is_space_admin(&exploitant, &espace).await);
    }

    /// Une compétition introuvable refuse : échec fermé.
    #[sqlx::test]
    async fn une_competition_introuvable_refuse(pool: PgPool) {
        let coach: CoachId = EntityId::new();
        let inconnue: CompetitionId = EntityId::new();
        assert!(!adapter(&pool).is_competition_admin(&coach, &inconnue).await);
    }
}
