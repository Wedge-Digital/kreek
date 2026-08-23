use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::membership::{NombreAdministrateurs, SpaceMembershipError};
use crate::app::spaces::domain::space_repository_port::space_repository_port::{
    ISpaceRepository, SpaceRepositoryError,
};
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct RemoveMemberCommand {
    pub space_id: SpaceId,
    pub acteur: CoachId,
    pub cible: CoachId,
}

#[derive(Debug)]
pub enum RemoveMemberError {
    EspaceInconnu,
    Metier(SpaceMembershipError),
    Database(String),
}

impl From<SpaceRepositoryError> for RemoveMemberError {
    fn from(e: SpaceRepositoryError) -> Self {
        RemoveMemberError::Database(e.to_string())
    }
}

impl From<SpaceMembershipError> for RemoveMemberError {
    fn from(e: SpaceMembershipError) -> Self {
        RemoveMemberError::Metier(e)
    }
}

/// Le compte sert ici aussi : retirer un administrateur peut faire passer
/// l'espace à un seul, ce qui fige la ligne du survivant.
///
/// C'est le seul des trois événements d'appartenance à franchir la frontière —
/// un coach retiré peut être administrateur d'une compétition de l'espace. Le
/// use case l'ignore : il émet sur le bus **interne**, et c'est le publisher qui
/// convertit. L'`app_event_bus` n'est paramètre d'aucun use case.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RemoveMemberCommand,
    repo: &dyn ISpaceRepository,
    bus: &EventBus,
) -> Result<NombreAdministrateurs, RemoveMemberError> {
    let mut space = repo
        .find_by_id(&cmd.space_id)
        .await?
        .ok_or(RemoveMemberError::EspaceInconnu)?;

    let changement = space.remove_member(&cmd.acteur, &cmd.cible)?;

    repo.delete_member(&cmd.space_id, &cmd.cible).await?;

    if let Some(evenement) = changement.evenement {
        emettre(bus, evenement.to_enveloppe());
    }

    Ok(changement.administrateurs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::identity::authorization::SpaceProfile;
    use crate::app::spaces::use_cases::test_doubles::{coach, espace, FakeSpaceRepo};
    use crate::common::services::event_bus::event_bus::new_bus;

    fn cmd(space_id: SpaceId, acteur: CoachId, cible: CoachId) -> RemoveMemberCommand {
        RemoveMemberCommand {
            space_id,
            acteur,
            cible,
        }
    }

    #[tokio::test]
    async fn retirer_un_membre_supprime_et_emet() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, membre]));
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let n = execute(cmd(sid, a, m), &repo, &bus).await.unwrap();

        assert_eq!(n, NombreAdministrateurs::try_new(1).unwrap());
        assert_eq!(repo.ecritures(), 1);
        assert_eq!(
            rx.try_recv().unwrap().event_type,
            "UserUnsubscribedFromSpace"
        );
    }

    #[tokio::test]
    async fn retirer_le_dernier_admin_n_ecrit_rien_et_n_emet_rien() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, membre]));
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let r = execute(cmd(sid, m, a), &repo, &bus).await;

        assert!(matches!(
            r,
            Err(RemoveMemberError::Metier(
                SpaceMembershipError::DernierAdministrateur
            ))
        ));
        assert_eq!(
            repo.ecritures(),
            0,
            "une suppression a eu lieu malgré le refus"
        );
        assert!(
            rx.try_recv().is_err(),
            "un événement a été émis malgré le refus"
        );
    }

    /// L'invariant ne porte que sur les administrateurs : retirer un membre
    /// ordinaire d'un espace qui n'en a qu'un doit réussir.
    #[tokio::test]
    async fn retirer_un_membre_ordinaire_d_un_espace_a_un_seul_admin_reussit() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, membre]));
        let bus = new_bus();

        let n = execute(cmd(sid, a, m), &repo, &bus).await.unwrap();

        assert_eq!(n, NombreAdministrateurs::try_new(1).unwrap());
        assert_eq!(repo.ecritures(), 1);
    }

    #[tokio::test]
    async fn l_acteur_ne_se_retire_pas_lui_meme() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let autre = coach("Autre", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, autre]));
        let bus = new_bus();

        let r = execute(cmd(sid, a, a), &repo, &bus).await;

        assert!(matches!(
            r,
            Err(RemoveMemberError::Metier(
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

        let r = execute(cmd(sid, a, CoachId::new()), &repo, &bus).await;

        assert!(matches!(
            r,
            Err(RemoveMemberError::Metier(SpaceMembershipError::PasMembre))
        ));
        assert_eq!(repo.ecritures(), 0);
    }

    #[tokio::test]
    async fn un_espace_inconnu_est_refuse() {
        let repo = FakeSpaceRepo::vide();
        let bus = new_bus();

        let r = execute(
            cmd(SpaceId::new(), CoachId::new(), CoachId::new()),
            &repo,
            &bus,
        )
        .await;

        assert!(matches!(r, Err(RemoveMemberError::EspaceInconnu)));
    }
}
