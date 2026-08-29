//! Ce que les deux use cases de points manuels partagent : leur erreur et leur
//! contrôle d'autorisation.
//!
//! **Sans suffixe `_use_case`** parce que ce n'en est pas un — la convention du
//! `CLAUDE.md` réserve ce suffixe aux orchestrations, et un fichier qui n'en
//! porte aucune ne doit pas s'en réclamer.

use crate::app::ranking::ports::{IRankingAdminPort, RankingRepositoryError};

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

/// Les deux portes d'entrée, que la carte 426 a montré qu'il faut garder
/// distinctes : dans `competitions`, un `||` identique avait rendu invisible la
/// suppression de l'une des deux branches, faute de tests qui les séparent.
///
/// **Instrumentée, et non déclarée `arch:no-instrument`.** Le port sépare les
/// deux sources d'autorisation précisément pour qu'on sache laquelle a répondu ;
/// un journal muet lui ôterait la moitié de sa valeur. Les `ret` des deux
/// appels laissent la trace en production, là où le seul `Forbidden` du use case
/// ne dirait pas *pourquoi* — ni, en cas d'accès inattendu, *par où*.
// L'attribut tient sur une ligne : l'axe 11 ne regarde que la ligne qui précède
// la signature, et `cargo fmt` replierait un attribut plus long sur `)]` — que
// le contrôle ne reconnaît pas. Les identifiants du contexte sont de toute
// façon dans le `debug!` ci-dessous.
#[tracing::instrument(skip_all, fields(user_id = %user_id), ret)]
pub async fn autorise(
    admin: &dyn IRankingAdminPort,
    user_id: &str,
    competition_id: &str,
    space_id: &str,
) -> bool {
    // Les deux sont évaluées, sans court-circuit : un `||` sauterait le second
    // appel dès que le premier répond vrai, et le journal ne dirait plus que
    // l'accès était de toute façon acquis par l'autre porte. Deux lectures
    // coûtent moins que l'ambiguïté qu'un court-circuit installe dans la trace.
    let par_competition = admin.is_competition_admin(user_id, competition_id).await;
    let par_espace = admin.is_space_admin(user_id, space_id).await;
    tracing::debug!(par_competition, par_espace, "autorisation points manuels");
    par_competition || par_espace
}
