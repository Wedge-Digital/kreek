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

use crate::app::shared_kernel::bloodbowl::admin_access::{is_admin, IAdminAccessPort};
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

/// Admin de la compétition, ou admin de l'espace — la règle de tout kreek,
/// `is_admin` (carte 572).
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
    is_admin(access, coach_id, space_id, Some(competition_id)).await
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
    async fn competition_admin_only() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new().competition_admin(&coach, &competition);
        assert!(peut_administrer(&port, &coach, &space, &competition).await);
    }

    /// Le cas de la carte 544 : aucun droit sur la compétition, mais le
    /// gouvernail de l'espace.
    #[tokio::test]
    async fn space_admin_only() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new().space_admin(&coach, &space);
        assert!(peut_administrer(&port, &coach, &space, &competition).await);
    }

    #[tokio::test]
    async fn admin_of_both() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new()
            .space_admin(&coach, &space)
            .competition_admin(&coach, &competition);
        assert!(peut_administrer(&port, &coach, &space, &competition).await);
    }

    #[tokio::test]
    async fn neither() {
        let (coach, space, competition) = ids();
        let port = FakeAdminAccess::new();
        assert!(!peut_administrer(&port, &coach, &space, &competition).await);
    }

    /// L'admin d'une autre compétition n'administre pas celle-ci.
    #[tokio::test]
    async fn admin_of_another_competition() {
        let (coach, space, competition) = ids();
        let other: CompetitionId = EntityId::new();
        let port = FakeAdminAccess::new().competition_admin(&coach, &other);
        assert!(!peut_administrer(&port, &coach, &space, &competition).await);
    }
}
