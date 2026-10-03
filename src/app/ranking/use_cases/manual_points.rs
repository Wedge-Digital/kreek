//! Ce que les deux use cases de points manuels partagent : leur erreur et leur
//! contrôle d'autorisation.
//!
//! **Sans suffixe `_use_case`** parce que ce n'en est pas un — la convention du
//! `CLAUDE.md` réserve ce suffixe aux orchestrations, et un fichier qui n'en
//! porte aucune ne doit pas s'en réclamer.

use crate::app::ranking::ports::RankingRepositoryError;
use crate::app::shared_kernel::bloodbowl::admin_access::{est_admin, IAdminAccessPort};
use crate::app::shared_kernel::identity::ids::EntityId;

/// **Pas de variante `Invalid`.** Les value objects de la carte 449 valident à
/// la construction : le handler ne peut pas fabriquer une commande invalide.
/// Revalider ici ferait le travail deux fois, et les deux copies finiraient par
/// diverger — c'est toujours la seconde qu'on oublie.
#[derive(Debug, PartialEq, Eq)]
pub enum ManualPointsError {
    Forbidden,
    TeamNotEnrolled,
    NotFound,
    Repository(String),
}

impl From<RankingRepositoryError> for ManualPointsError {
    fn from(e: RankingRepositoryError) -> Self {
        Self::Repository(e.to_string())
    }
}

/// Qui peut attribuer ou retirer des points manuels : un admin de l'espace ou
/// de la compétition — la règle commune à tout kreek (carte 570).
///
/// Les deux portes restaient évaluées séparément, pour que la trace dise
/// laquelle avait ouvert. `est_admin` court-circuite : l'espace répond d'abord,
/// et la compétition n'est interrogée qu'à défaut. Ce que la carte 426 avait
/// payé — une branche supprimée sans qu'aucun test ne rougisse — est désormais
/// tenu par les tests du service commun, qui exercent chaque porte seule.
///
/// Un identifiant illisible refuse : un contrôle d'accès échoue fermé.
// L'attribut tient sur une ligne : l'axe 11 ne regarde que la ligne qui précède
// la signature, et `cargo fmt` replierait un attribut plus long sur `)]` — que
// le contrôle ne reconnaît pas.
#[tracing::instrument(skip_all, fields(user_id = %user_id), ret)]
pub async fn autorise(
    admin: &dyn IAdminAccessPort,
    user_id: &str,
    competition_id: &str,
    space_id: &str,
) -> bool {
    let (Ok(user), Ok(competition), Ok(space)) = (
        EntityId::try_new(user_id),
        EntityId::try_new(competition_id),
        EntityId::try_new(space_id),
    ) else {
        return false;
    };
    est_admin(admin, &user, &space, Some(&competition)).await
}
