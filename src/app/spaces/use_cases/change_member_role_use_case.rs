use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::membership::{NombreAdministrateurs, SpaceMembershipError};
use crate::app::spaces::domain::space_repository_port::space_repository_port::{
    ISpaceRepository, SpaceRepositoryError,
};
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

/// L'acteur vient de la session, jamais du formulaire : deux des règles portent
/// sur lui, et une identité qui transite par le client est réécrivable.
#[derive(Debug)]
pub struct ChangeMemberRoleCommand {
    pub space_id: SpaceId,
    pub acteur: CoachId,
    pub cible: CoachId,
    pub nouveau_profil: SpaceProfile,
}

#[derive(Debug)]
pub enum ChangeMemberRoleError {
    EspaceInconnu,
    /// Transportée telle quelle depuis le domaine, jamais réinterprétée : c'est
    /// le contrôleur qui choisira le statut HTTP.
    Metier(SpaceMembershipError),
    Database(String),
}

impl From<SpaceRepositoryError> for ChangeMemberRoleError {
    fn from(e: SpaceRepositoryError) -> Self {
        ChangeMemberRoleError::Database(e.to_string())
    }
}

impl From<SpaceMembershipError> for ChangeMemberRoleError {
    fn from(e: SpaceMembershipError) -> Self {
        ChangeMemberRoleError::Metier(e)
    }
}

/// Rend le nombre d'administrateurs **postérieur** au changement.
///
/// Le contrôleur en a besoin pour re-rendre la ligne : rétrograder
/// l'avant-dernier administrateur fige le sélecteur du dernier, et lui seul le
/// sait. Le compte est lu sur l'agrégat muté, pas relu en base — l'agrégat vient
/// d'appliquer le changement, c'est la source la plus fraîche qui soit.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ChangeMemberRoleCommand,
    repo: &dyn ISpaceRepository,
    bus: &EventBus,
) -> Result<NombreAdministrateurs, ChangeMemberRoleError> {
    let mut space = repo
        .find_by_id(&cmd.space_id)
        .await?
        .ok_or(ChangeMemberRoleError::EspaceInconnu)?;

    let changement =
        space.change_member_role(&cmd.acteur, &cmd.cible, cmd.nouveau_profil.clone())?;

    // Reposter le rôle courant réussit sans rien changer : ni écriture, ni
    // événement. L'absence d'événement est le signal, et il vient du domaine.
    if let Some(evenement) = changement.evenement {
        repo.update_member_profile(&cmd.space_id, &cmd.cible, &cmd.nouveau_profil)
            .await?;
        emettre(bus, evenement.to_enveloppe());
    }

    Ok(changement.administrateurs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::spaces::use_cases::test_doubles::{coach, espace, FakeSpaceRepo};
    use crate::common::services::event_bus::event_bus::new_bus;

    fn cmd(
        space_id: SpaceId,
        acteur: CoachId,
        cible: CoachId,
        p: SpaceProfile,
    ) -> ChangeMemberRoleCommand {
        ChangeMemberRoleCommand {
            space_id,
            acteur,
            cible,
            nouveau_profil: p,
        }
    }

    #[tokio::test]
    async fn promouvoir_un_membre_ecrit_et_emet() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, membre]));
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let n = execute(cmd(sid, a, m, SpaceProfile::SpaceAdmin), &repo, &bus)
            .await
            .unwrap();

        assert_eq!(n, NombreAdministrateurs::try_new(2).unwrap());
        assert_eq!(repo.ecritures(), 1);
        assert_eq!(
            rx.try_recv().unwrap().event_type,
            "UserPromotedToSpaceAdmin"
        );
    }

    #[tokio::test]
    async fn retrograder_un_admin_parmi_deux_ecrit_et_emet() {
        let a1 = coach("Admin1", SpaceProfile::SpaceAdmin);
        let a2 = coach("Admin2", SpaceProfile::SpaceAdmin);
        let (i1, i2) = (a1.id, a2.id);
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![a1, a2]));
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let n = execute(cmd(sid, i1, i2, SpaceProfile::SpaceUser), &repo, &bus)
            .await
            .unwrap();

        assert_eq!(n, NombreAdministrateurs::try_new(1).unwrap());
        assert_eq!(repo.ecritures(), 1);
        assert_eq!(rx.try_recv().unwrap().event_type, "UserDemotedToSpaceUser");
    }

    /// Vérifier le type d'erreur ne suffit pas : une implémentation qui
    /// écrirait d'abord et validerait ensuite rendrait la même erreur, en ayant
    /// modifié la base. D'où le compteur d'écritures et le bus lu.
    #[tokio::test]
    async fn retrograder_le_dernier_admin_n_ecrit_rien_et_n_emet_rien() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, membre]));
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let r = execute(cmd(sid, m, a, SpaceProfile::SpaceUser), &repo, &bus).await;

        assert!(matches!(
            r,
            Err(ChangeMemberRoleError::Metier(
                SpaceMembershipError::DernierAdministrateur
            ))
        ));
        assert_eq!(
            repo.ecritures(),
            0,
            "une écriture a eu lieu malgré le refus"
        );
        assert!(
            rx.try_recv().is_err(),
            "un événement a été émis malgré le refus"
        );
    }

    #[tokio::test]
    async fn l_acteur_ne_change_pas_son_propre_role() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let autre = coach("Autre", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, autre]));
        let bus = new_bus();

        let r = execute(cmd(sid, a, a, SpaceProfile::SpaceUser), &repo, &bus).await;

        assert!(matches!(
            r,
            Err(ChangeMemberRoleError::Metier(
                SpaceMembershipError::ActeurEstLaCible
            ))
        ));
        assert_eq!(repo.ecritures(), 0);
    }

    #[tokio::test]
    async fn une_cible_non_membre_est_refusee() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin]));
        let bus = new_bus();

        let r = execute(
            cmd(sid, a, CoachId::new(), SpaceProfile::SpaceAdmin),
            &repo,
            &bus,
        )
        .await;

        assert!(matches!(
            r,
            Err(ChangeMemberRoleError::Metier(
                SpaceMembershipError::PasMembre
            ))
        ));
        assert_eq!(repo.ecritures(), 0);
    }

    #[tokio::test]
    async fn un_espace_inconnu_est_refuse() {
        let repo = FakeSpaceRepo::vide();
        let bus = new_bus();

        let r = execute(
            cmd(
                SpaceId::new(),
                CoachId::new(),
                CoachId::new(),
                SpaceProfile::SpaceAdmin,
            ),
            &repo,
            &bus,
        )
        .await;

        assert!(matches!(r, Err(ChangeMemberRoleError::EspaceInconnu)));
    }

    /// Reposter le rôle courant réussit — et ne doit **ni écrire, ni émettre**.
    /// Le journal n'enregistre pas un changement qui n'a pas eu lieu.
    #[tokio::test]
    async fn reposter_le_role_courant_reussit_sans_ecrire_ni_emettre() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, membre]));
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let n = execute(cmd(sid, a, m, SpaceProfile::SpaceUser), &repo, &bus)
            .await
            .unwrap();

        assert_eq!(n, NombreAdministrateurs::try_new(1).unwrap());
        assert_eq!(
            repo.ecritures(),
            0,
            "rien n'a changé, rien ne doit s'écrire"
        );
        assert!(
            rx.try_recv().is_err(),
            "rien n'a changé, rien ne doit s'émettre"
        );
    }
}
