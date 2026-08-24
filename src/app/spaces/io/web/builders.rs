//! Construction des vues de l'administration d'espace.
//!
//! Ici et non par un `from_domain()` co-localisé : `MemberRowVm` se bâtit à
//! partir d'un **DTO de port** — `SpaceMemberRow` — et non d'un objet du
//! domaine. C'est la règle du projet, et elle tient `view_models` à l'écart des
//! types du port.

use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::app::spaces::domain::space_repository_port::space_repository_port::{
    CandidateRow, SpaceMemberRow,
};
use crate::common::initials::initials;

pub struct MemberRowVm {
    pub coach_id: String,
    pub name: String,
    pub email: String,
    pub initials: String,
    pub is_self: bool,
    pub is_admin: bool,
    /// Le sélecteur de rôle est figé.
    pub role_locked: bool,
    /// Le bouton de retrait est absent.
    pub removable: bool,
}

/// Bâtit les lignes de la liste des membres.
///
/// `role_locked` et `removable` sont **des politesses, pas des gardes**. Ils
/// évitent que l'interface propose ce qu'elle refusera ; la règle qui fait foi
/// vit dans `Space::change_member_role` et `Space::remove_member`, et un client
/// qui contournerait le grisage se ferait refuser par le domaine.
///
/// C'est la répartition qu'impose le projet — le front grise, le domaine refuse
/// — et elle se vérifie par un POST direct, sans passer par l'interface.
///
/// # La clause « dernier administrateur » est redondante à l'écran
///
/// Elle ne s'applique jamais qu'à sa propre ligne, que `is_self` fige déjà.
/// Pour qu'elle joue seule il faudrait une cible seule administratrice, un
/// spectateur distinct d'elle, et ce spectateur administrateur — sinon la page
/// rend 403. Le spectateur serait donc un second administrateur, et la cible ne
/// serait plus seule.
///
/// Constaté en écrivant le test e2e qui devait l'observer (carte 374). C'est le
/// même raisonnement qui a montré, en carte 371, que `DernierAdministrateur`
/// est inatteignable depuis le web.
///
/// La clause reste, et sa place est ici : elle décrit la règle, pas le seul cas
/// où elle mord aujourd'hui. Si la page s'ouvrait un jour à un profil non
/// administrateur — un compte d'exploitation, par exemple — elle jouerait.
pub fn build_member_rows(lignes: Vec<SpaceMemberRow>, moi: &CoachId) -> Vec<MemberRowVm> {
    let admins = lignes
        .iter()
        .filter(|l| l.profile == SpaceProfile::SpaceAdmin.as_str())
        .count();

    lignes
        .into_iter()
        .map(|l| {
            let is_admin = l.profile == SpaceProfile::SpaceAdmin.as_str();
            let is_self = l.coach_id == moi.to_string();
            let dernier_admin = is_admin && admins == 1;
            MemberRowVm {
                initials: initials(&l.coach_name),
                is_self,
                is_admin,
                role_locked: is_self || dernier_admin,
                removable: !is_self && !dernier_admin,
                coach_id: l.coach_id,
                name: l.coach_name,
                email: l.email,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(id: &str, nom: &str, profil: SpaceProfile) -> SpaceMemberRow {
        SpaceMemberRow {
            coach_id: id.to_string(),
            coach_name: nom.to_string(),
            email: format!("{nom}@bb.club"),
            icon: None,
            profile: profil.as_str().to_string(),
        }
    }

    fn id(n: &str) -> CoachId {
        CoachId::try_new(n).unwrap()
    }

    const A: &str = "01JAAAAAAAAAAAAAAAAAAAAAAA";
    const B: &str = "01JBBBBBBBBBBBBBBBBBBBBBBB";
    const C: &str = "01JCCCCCCCCCCCCCCCCCCCCCCC";

    #[test]
    fn le_seul_administrateur_a_son_selecteur_fige_et_aucun_retrait() {
        let vms = build_member_rows(
            vec![
                ligne(A, "Chef", SpaceProfile::SpaceAdmin),
                ligne(B, "Membre", SpaceProfile::SpaceUser),
            ],
            &id(C),
        );

        assert!(vms[0].role_locked, "le dernier administrateur est figé");
        assert!(!vms[0].removable);
        assert!(!vms[1].role_locked, "le membre ordinaire reste modifiable");
        assert!(vms[1].removable);
    }

    #[test]
    fn deux_administrateurs_sont_l_un_et_l_autre_modifiables() {
        let vms = build_member_rows(
            vec![
                ligne(A, "Chef", SpaceProfile::SpaceAdmin),
                ligne(B, "Adjoint", SpaceProfile::SpaceAdmin),
            ],
            &id(C),
        );

        assert!(vms.iter().all(|v| !v.role_locked));
        assert!(vms.iter().all(|v| v.removable));
    }

    /// Sa propre ligne est figée quel que soit le nombre d'administrateurs :
    /// c'est une règle sur l'acteur, pas sur l'invariant.
    #[test]
    fn sa_propre_ligne_est_figee_et_non_retirable() {
        let vms = build_member_rows(
            vec![
                ligne(A, "Chef", SpaceProfile::SpaceAdmin),
                ligne(B, "Adjoint", SpaceProfile::SpaceAdmin),
            ],
            &id(A),
        );

        assert!(vms[0].is_self);
        assert!(vms[0].role_locked);
        assert!(!vms[0].removable);
        assert!(!vms[1].role_locked, "l'autre reste modifiable");
    }

    #[test]
    fn les_initiales_viennent_du_pseudo() {
        let vms = build_member_rows(
            vec![ligne(A, "Colonel Castor", SpaceProfile::SpaceUser)],
            &id(B),
        );
        assert_eq!(vms[0].initials, "CC");
    }
}

/// Une ligne de la liste des candidats à l'ajout direct.
///
/// `est_membre` décide de tout le rendu de la ligne : un badge, ou un sélecteur
/// de profil et un bouton. Le gabarit ne tranche rien de lui-même.
pub struct CandidateRowVm {
    pub coach_id: String,
    pub name: String,
    pub email: String,
    pub initials: String,
    pub est_membre: bool,
}

pub fn build_candidate_rows(lignes: Vec<CandidateRow>) -> Vec<CandidateRowVm> {
    lignes
        .into_iter()
        .map(|l| CandidateRowVm {
            initials: initials(&l.coach_name),
            coach_id: l.coach_id,
            name: l.coach_name,
            email: l.email,
            est_membre: l.est_membre,
        })
        .collect()
}
