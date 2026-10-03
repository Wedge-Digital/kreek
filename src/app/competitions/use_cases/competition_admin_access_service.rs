//! Qui peut administrer une compétition.
//!
//! # Une seule règle, deux appelants
//!
//! « Administrateur de la compétition, ou administrateur de l'espace » gardait
//! l'entrée dans `require_admin_access` et décidait de l'affichage du bouton
//! dans `competition_detail` — deux écritures, dont la seconde ne portait que
//! la première moitié. L'administrateur d'espace avait donc le droit d'entrer
//! sans qu'on le lui montre (carte 544).
//!
//! Les deux appellent désormais cette fonction. Elle ne décide **pas** de
//! l'accès à elle seule : `require_admin_access` lui ajoute la cohérence du
//! couple saison/compétition (carte 416), qui ne relève pas du droit mais du
//! chemin.

use crate::app::shared_kernel::bloodbowl::admin_access::{est_admin, IAdminAccessPort};
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

/// Admin de la compétition, ou admin de l'espace — la règle de tout kreek,
/// `est_admin` (carte 572).
///
/// Elle lisait les admins de la compétition dans une liste déjà en mémoire, et
/// comparait aussi le nom du coach : une compétition était censée pouvoir
/// désigner ses admins par l'un ou l'autre. Ils ne s'enregistrent en fait que
/// par identifiant, et le nom n'était que leur pseudonyme obtenu par jointure.
/// Le service commun relit la compétition : une lecture de plus par contrôle,
/// le prix d'une règle unique.
// Sur une seule ligne : l'axe 11 n'examine que celle qui précède la fonction.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn peut_administrer(
    access: &dyn IAdminAccessPort,
    coach_id: &CoachId,
    space_id: &SpaceId,
    competition_id: &CompetitionId,
) -> bool {
    est_admin(access, coach_id, space_id, Some(competition_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::identity::ids::EntityId;

    fn ids() -> (CoachId, SpaceId, CompetitionId) {
        (
            CoachId::try_new("01JCAAAAAAAAAAAAAAAAAAAAAA").unwrap(),
            SpaceId::try_new("01JSPBBBBBBBBBBBBBBBBBBBBB").unwrap(),
            EntityId::new(),
        )
    }

    // ── Les quatre combinaisons ──────────────────────────────────────────────

    #[tokio::test]
    async fn admin_de_la_competition_seulement() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new().admin_competition(&coach, &competition);
        assert!(peut_administrer(&port, &coach, &espace, &competition).await);
    }

    /// Le cas de la carte 544 : aucun droit sur la compétition, mais le
    /// gouvernail de l'espace.
    #[tokio::test]
    async fn admin_de_l_espace_seulement() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new().admin_espace(&coach, &espace);
        assert!(peut_administrer(&port, &coach, &espace, &competition).await);
    }

    #[tokio::test]
    async fn admin_des_deux() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new()
            .admin_espace(&coach, &espace)
            .admin_competition(&coach, &competition);
        assert!(peut_administrer(&port, &coach, &espace, &competition).await);
    }

    #[tokio::test]
    async fn ni_l_un_ni_l_autre() {
        let (coach, espace, competition) = ids();
        let port = FakeAdminAccess::new();
        assert!(!peut_administrer(&port, &coach, &espace, &competition).await);
    }

    /// L'admin d'une autre compétition n'administre pas celle-ci.
    #[tokio::test]
    async fn admin_d_une_autre_competition() {
        let (coach, espace, competition) = ids();
        let autre: CompetitionId = EntityId::new();
        let port = FakeAdminAccess::new().admin_competition(&coach, &autre);
        assert!(!peut_administrer(&port, &coach, &espace, &competition).await);
    }
}
