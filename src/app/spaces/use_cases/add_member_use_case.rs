use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::coach::Coach;
use crate::app::spaces::domain::membership::{
    NombreAdministrateurs, Notification, SpaceMembershipError,
};
use crate::app::spaces::domain::space_repository_port::space_repository_port::{
    ISpaceRepository, SpaceRepositoryError,
};
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::ISpaceUserCacheRepository;
use crate::common::services::email::IEmailService;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use askama::Template;

#[derive(Template)]
#[template(path = "emails/fr_FR/space_member_added.html")]
struct CourtoisieEmail {
    coach_name: String,
    admin_name: String,
    space_name: String,
    space_url: String,
    app_url: String,
}

#[derive(Debug)]
pub struct AddMemberCommand {
    pub space_id: SpaceId,
    pub acteur: CoachId,
    pub nouveau: CoachId,
    pub profil: SpaceProfile,
    pub notifier: Notification,
    /// L'adresse de l'espace, fournie par l'appelant : ce BC est extractible et
    /// ne connaît pas les URL de son hôte.
    pub space_url: String,
    pub app_url: String,
}

#[derive(Debug)]
pub enum AddMemberError {
    EspaceInconnu,
    CoachInconnu,
    Metier(SpaceMembershipError),
    Database(String),
}

impl From<SpaceRepositoryError> for AddMemberError {
    fn from(e: SpaceRepositoryError) -> Self {
        AddMemberError::Database(e.to_string())
    }
}

impl From<SpaceMembershipError> for AddMemberError {
    fn from(e: SpaceMembershipError) -> Self {
        AddMemberError::Metier(e)
    }
}

/// Ajoute un coach à un espace, sur ordre d'un administrateur.
///
/// Le `Coach` est bâti depuis le cache d'utilisateurs : l'agrégat en stocke, et
/// un identifiant seul ne permet pas d'en construire un.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: AddMemberCommand,
    repo: &dyn ISpaceRepository,
    cache: &dyn ISpaceUserCacheRepository,
    email: &dyn IEmailService,
    bus: &EventBus,
) -> Result<NombreAdministrateurs, AddMemberError> {
    let mut space = repo
        .find_by_id(&cmd.space_id)
        .await?
        .ok_or(AddMemberError::EspaceInconnu)?;

    let user = cache
        .find_user_by_id(&cmd.nouveau)
        .await
        .map_err(|_| AddMemberError::CoachInconnu)?;

    let acteur_nom = cache
        .find_user_by_id(&cmd.acteur)
        .await
        .map(|u| u.name.to_string())
        .unwrap_or_else(|_| "un administrateur".to_string());

    let nouveau = Coach::new(
        user.id,
        user.name.clone(),
        cmd.profil.clone(),
        user.icon.clone(),
    );
    let espace_nom = space.name().to_string();
    let changement = space.add_member(&cmd.acteur, nouveau)?;

    repo.add_member(&cmd.space_id, &cmd.nouveau, &cmd.profil)
        .await?;

    if cmd.notifier == Notification::Envoyer {
        prevenir(email, &user, &acteur_nom, &espace_nom, &cmd).await;
    }

    if let Some(evenement) = changement.evenement {
        emettre(bus, evenement.to_enveloppe());
    }

    Ok(changement.administrateurs)
}

/// La courtoisie, et **son échec n'annule rien**.
///
/// L'appartenance est posée, l'événement sera émis, et un email qui ne part pas
/// est journalisé en `warn`. Refuser l'ajout parce que le serveur de mail est
/// indisponible ferait dépendre une règle d'appartenance d'un service qui n'en
/// gouverne aucune.
///
/// C'est l'inverse du choix fait pour la création de compte, où l'email est
/// l'unique porte d'entrée — et les deux sont cohérents : là-bas un accès, ici
/// un agrément.
async fn prevenir(
    email: &dyn IEmailService,
    user: &crate::app::spaces::domain::user::User,
    acteur_nom: &str,
    espace_nom: &str,
    cmd: &AddMemberCommand,
) {
    let corps = CourtoisieEmail {
        coach_name: user.name.to_string(),
        admin_name: acteur_nom.to_string(),
        space_name: espace_nom.to_string(),
        space_url: cmd.space_url.clone(),
        app_url: cmd.app_url.clone(),
    };
    let Ok(html) = corps.render() else {
        tracing::warn!("add_member: rendu de la courtoisie impossible");
        return;
    };
    if let Err(e) = email
        // arch:ok envoi d'e-mail, pas une émission d'événement de domaine
        .send(
            vec![user.email.as_ref().to_string()],
            format!("Tu fais partie de {espace_nom}"),
            html,
        )
        .await
    {
        tracing::warn!(coach = %user.id, "add_member: courtoisie non envoyée: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::spaces::use_cases::test_doubles::{
        coach, espace, user, FakeEmail, FakeSpaceRepo, FakeUserCache,
    };
    use crate::common::services::event_bus::event_bus::new_bus;

    fn cmd(
        space_id: SpaceId,
        acteur: CoachId,
        nouveau: CoachId,
        profil: SpaceProfile,
        n: Notification,
    ) -> AddMemberCommand {
        AddMemberCommand {
            space_id,
            acteur,
            nouveau,
            profil,
            notifier: n,
            space_url: "https://exemple.test/app/x".into(),
            app_url: "https://exemple.test".into(),
        }
    }

    #[tokio::test]
    async fn ajouter_un_coach_ecrit_et_emet() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let sid = SpaceId::new();
        let nouveau = CoachId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin]));
        let cache = FakeUserCache::avec(vec![user(a, "Admin"), user(nouveau, "Nouveau")]);
        let email = FakeEmail::qui_marche();
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let n = execute(
            cmd(
                sid,
                a,
                nouveau,
                SpaceProfile::SpaceUser,
                Notification::Taire,
            ),
            &repo,
            &cache,
            &email,
            &bus,
        )
        .await
        .unwrap();

        assert_eq!(n, NombreAdministrateurs::try_new(1).unwrap());
        assert_eq!(repo.ecritures(), 1);
        assert_eq!(rx.try_recv().unwrap().event_type, "UserAddedToSpaceByAdmin");
    }

    #[tokio::test]
    async fn ajouter_un_administrateur_incremente_le_compte() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let sid = SpaceId::new();
        let nouveau = CoachId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin]));
        let cache = FakeUserCache::avec(vec![user(a, "Admin"), user(nouveau, "Second")]);
        let email = FakeEmail::qui_marche();
        let bus = new_bus();

        let n = execute(
            cmd(
                sid,
                a,
                nouveau,
                SpaceProfile::SpaceAdmin,
                Notification::Taire,
            ),
            &repo,
            &cache,
            &email,
            &bus,
        )
        .await
        .unwrap();

        assert_eq!(n, NombreAdministrateurs::try_new(2).unwrap());
    }

    #[tokio::test]
    async fn un_coach_deja_membre_n_ecrit_rien_n_emet_rien_et_n_envoie_rien() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let membre = coach("Membre", SpaceProfile::SpaceUser);
        let (a, m) = (admin.id, membre.id);
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin, membre]));
        let cache = FakeUserCache::avec(vec![user(a, "Admin"), user(m, "Membre")]);
        let email = FakeEmail::qui_marche();
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let r = execute(
            cmd(sid, a, m, SpaceProfile::SpaceUser, Notification::Envoyer),
            &repo,
            &cache,
            &email,
            &bus,
        )
        .await;

        assert!(matches!(
            r,
            Err(AddMemberError::Metier(SpaceMembershipError::DejaMembre))
        ));
        assert_eq!(
            repo.ecritures(),
            0,
            "une écriture a eu lieu malgré le refus"
        );
        assert_eq!(email.envoyes(), 0, "un email est parti malgré le refus");
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn un_espace_inconnu_est_refuse() {
        let repo = FakeSpaceRepo::vide();
        let cache = FakeUserCache::avec(vec![]);
        let email = FakeEmail::qui_marche();
        let bus = new_bus();

        let r = execute(
            cmd(
                SpaceId::new(),
                CoachId::new(),
                CoachId::new(),
                SpaceProfile::SpaceUser,
                Notification::Taire,
            ),
            &repo,
            &cache,
            &email,
            &bus,
        )
        .await;

        assert!(matches!(r, Err(AddMemberError::EspaceInconnu)));
    }

    /// Un coach absent du cache ne peut pas être ajouté : l'agrégat stocke des
    /// `Coach`, et sans pseudo il n'y a pas de `Coach` à construire.
    #[tokio::test]
    async fn un_coach_absent_du_cache_est_refuse() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let sid = SpaceId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin]));
        let cache = FakeUserCache::avec(vec![user(a, "Admin")]);
        let email = FakeEmail::qui_marche();
        let bus = new_bus();

        let r = execute(
            cmd(
                sid,
                a,
                CoachId::new(),
                SpaceProfile::SpaceUser,
                Notification::Taire,
            ),
            &repo,
            &cache,
            &email,
            &bus,
        )
        .await;

        assert!(matches!(r, Err(AddMemberError::CoachInconnu)));
        assert_eq!(repo.ecritures(), 0);
    }

    #[tokio::test]
    async fn la_courtoisie_part_si_elle_est_demandee_et_pas_sinon() {
        for (notif, attendu) in [(Notification::Envoyer, 1), (Notification::Taire, 0)] {
            let admin = coach("Admin", SpaceProfile::SpaceAdmin);
            let a = admin.id;
            let sid = SpaceId::new();
            let nouveau = CoachId::new();
            let repo = FakeSpaceRepo::avec(espace(sid, vec![admin]));
            let cache = FakeUserCache::avec(vec![user(a, "Admin"), user(nouveau, "Nouveau")]);
            let email = FakeEmail::qui_marche();
            let bus = new_bus();

            execute(
                cmd(sid, a, nouveau, SpaceProfile::SpaceUser, notif),
                &repo,
                &cache,
                &email,
                &bus,
            )
            .await
            .unwrap();

            assert_eq!(email.envoyes(), attendu, "pour {notif:?}");
        }
    }

    /// Le seul test qui vérifie que la courtoisie ne gouverne pas
    /// l'appartenance.
    ///
    /// Refuser l'ajout parce que le serveur de mail est indisponible ferait
    /// dépendre une règle d'appartenance d'un service qui n'en gouverne aucune.
    #[tokio::test]
    async fn un_envoi_qui_echoue_n_annule_pas_l_ajout() {
        let admin = coach("Admin", SpaceProfile::SpaceAdmin);
        let a = admin.id;
        let sid = SpaceId::new();
        let nouveau = CoachId::new();
        let repo = FakeSpaceRepo::avec(espace(sid, vec![admin]));
        let cache = FakeUserCache::avec(vec![user(a, "Admin"), user(nouveau, "Nouveau")]);
        let email = FakeEmail::en_panne();
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let r = execute(
            cmd(
                sid,
                a,
                nouveau,
                SpaceProfile::SpaceUser,
                Notification::Envoyer,
            ),
            &repo,
            &cache,
            &email,
            &bus,
        )
        .await;

        assert!(r.is_ok(), "l'ajout doit réussir malgré la panne d'email");
        assert_eq!(repo.ecritures(), 1, "et l'écriture avoir eu lieu");
        assert!(rx.try_recv().is_ok(), "et l'événement être parti");
    }
}
