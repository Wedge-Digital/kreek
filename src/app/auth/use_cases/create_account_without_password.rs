//! Créer un compte qu'un administrateur d'espace ajoute à son espace.
//!
//! `RegisterCommand` exige un mot de passe et refuse en dessous de huit
//! caractères ; un compte créé par un tiers n'en a pas, et lui en inventer un
//! serait pire — un mot de passe que personne ne connaît et que rien n'oblige à
//! changer.
//!
//! # L'e-mail est une étape, pas une option
//!
//! C'est l'**inverse** de la courtoisie envoyée par `spaces` lors d'un ajout :
//! là-bas un agrément, dont l'échec ne doit rien annuler. Ici c'est l'unique
//! porte d'entrée du compte. Un compte créé dont le lien n'est pas parti est un
//! compte inaccessible, qui occupe un pseudo et une adresse — et la tentative
//! suivante échouerait sur `PseudoDejaPris` sans que l'administrateur comprenne.
//!
//! D'où l'ordre : jeton, envoi, **puis** création. Si l'envoi échoue, aucun
//! compte n'a été créé et le jeton est effacé. L'administrateur réessaie sans
//! rien avoir à réparer.

use crate::app::auth::domain::domain_event::AuthDomainEvent::AccountCreated;
use crate::app::auth::domain::reset_token::Token;
use crate::app::auth::domain::user::User;
use crate::app::auth::io::repository::reset_token_repository::IResetTokenRepository;
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::auth::routes::path;
use crate::app::shared_kernel::identity::coach_name::CoachName;
use crate::app::shared_kernel::identity::email::Email;
use crate::app::shared_kernel::identity::ids::{EventId, UserId};
use crate::common::services::email::IEmailService;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;
use askama::Template;

/// Ce que porte `password_hash` pour un compte sans mot de passe.
///
/// La colonne est `NOT NULL`, et la rendre nullable toucherait le chemin de
/// connexion — bien au-delà de ce que cette carte engage.
///
/// La valeur reprend la convention de Django, d'où vient l'import legacy :
/// `PasswordHash::new` la refuse au parsing, donc **aucune session ne peut
/// s'ouvrir**. C'est exactement l'état des comptes importés, connu et maîtrisé,
/// plutôt qu'un état nouveau à faire connaître.
const SANS_MOT_DE_PASSE: &str = "!";

#[derive(Template)]
#[template(path = "emails/fr_FR/account_created_set_password.html")]
struct DefinirMotDePasseEmail {
    coach_name: String,
    reset_url: String,
}

#[derive(Debug)]
pub struct CreateAccountWithoutPasswordCommand {
    pub coach_name: String,
    pub email: String,
    /// L'URL publique, schéma compris — cf. `AuthContext::app_url`.
    pub app_url: String,
}

#[derive(Debug, PartialEq)]
pub enum CreateAccountError {
    PseudoInvalide(String),
    EmailInvalide(String),
    PseudoDejaPris,
    EmailDejaPris,
    EnvoiEmailImpossible(String),
    Database(String),
}

impl From<RepositoryError> for CreateAccountError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::CoachNameAlreadyTaken => CreateAccountError::PseudoDejaPris,
            RepositoryError::EmailAlreadyTaken => CreateAccountError::EmailDejaPris,
            RepositoryError::Database(m) => CreateAccountError::Database(m),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: CreateAccountWithoutPasswordCommand,
    users: &dyn IUserRepository,
    jetons: &dyn IResetTokenRepository,
    email: &dyn IEmailService,
    bus: &EventBus,
) -> Result<UserId, CreateAccountError> {
    let coach_name = CoachName::try_new(&cmd.coach_name)
        .map_err(|e| CreateAccountError::PseudoInvalide(e.to_string()))?;
    let adresse =
        Email::try_new(&cmd.email).map_err(|e| CreateAccountError::EmailInvalide(e.to_string()))?;

    // Le pseudo est vérifié **avant** l'envoi : sans ça, une faute de frappe sur
    // un pseudo existant enverrait un lien au titulaire actuel, qui n'a rien
    // demandé.
    //
    // L'unicité de l'adresse, elle, n'est connue qu'à l'insertion — le dépôt
    // n'offre pas de recherche par e-mail. Une adresse déjà prise reçoit donc un
    // lien qui ne mènera nulle part, le compte n'ayant pas été créé.
    if users.find_by_coach_name(&cmd.coach_name).await?.is_some() {
        return Err(CreateAccountError::PseudoDejaPris);
    }

    let jeton = Token::new();
    jetons.create(&jeton, &coach_name).await?;

    if let Err(e) = envoyer(email, &coach_name, &adresse, &jeton, &cmd.app_url).await {
        nettoyer(jetons, &jeton).await;
        return Err(e);
    }

    let user = User::new(
        UserId::new(),
        coach_name.clone(),
        None,
        adresse.clone(),
        SANS_MOT_DE_PASSE.to_string(),
    );
    if let Err(e) = users.create(&user).await {
        nettoyer(jetons, &jeton).await;
        return Err(e.into());
    }

    emettre(
        bus,
        AccountCreated {
            event_id: EventId::new(),
            user_id: user.id,
            user_name: coach_name,
            email: adresse,
        }
        .to_enveloppe(),
    );
    Ok(user.id)
}

async fn envoyer(
    email: &dyn IEmailService,
    coach_name: &CoachName,
    adresse: &Email,
    jeton: &Token,
    app_url: &str,
) -> Result<(), CreateAccountError> {
    // Rien à recoller : l'hôte injecte l'URL déjà normalisée.
    let reset_url = format!("{}{}/{}", app_url, path::RESET_PASSWORD_BASE, jeton);
    let html = DefinirMotDePasseEmail {
        coach_name: coach_name.to_string(),
        reset_url,
    }
    .render()
    .map_err(|e| CreateAccountError::EnvoiEmailImpossible(e.to_string()))?;

    email
        // arch:ok envoi d'e-mail, pas une émission d'événement de domaine
        .send(
            vec![adresse.value().to_string()],
            "Ton compte Bloodbowl Club t'attend".to_string(),
            html,
        )
        .await
        .map_err(|e| CreateAccountError::EnvoiEmailImpossible(e.to_string()))
}

/// Efface le jeton laissé derrière quand la suite échoue.
///
/// Sans ça, un jeton subsisterait pour un pseudo qui n'existe pas — et si
/// quelqu'un enregistrait ce pseudo plus tard, le lien déjà envoyé lui donnerait
/// la main sur le compte.
async fn nettoyer(jetons: &dyn IResetTokenRepository, jeton: &Token) {
    if let Err(e) = jetons.delete_by_token(&jeton.to_string()).await {
        tracing::warn!("create_account: jeton orphelin non effacé: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::auth::domain::reset_token::ResetToken;
    use crate::common::services::email::EmailError;
    use crate::common::services::event_bus::event_bus::new_bus;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakeUsers {
        pseudo_pris: bool,
        erreur_creation: Option<RepositoryError>,
        crees: AtomicUsize,
        dernier_hash: std::sync::Mutex<String>,
    }

    impl FakeUsers {
        fn libre() -> Self {
            Self {
                pseudo_pris: false,
                erreur_creation: None,
                crees: AtomicUsize::new(0),
                dernier_hash: std::sync::Mutex::new(String::new()),
            }
        }
        fn pseudo_pris() -> Self {
            Self {
                pseudo_pris: true,
                ..Self::libre()
            }
        }
        fn refuse(e: RepositoryError) -> Self {
            Self {
                erreur_creation: Some(e),
                ..Self::libre()
            }
        }
        fn crees(&self) -> usize {
            self.crees.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl IUserRepository for FakeUsers {
        async fn create(&self, user: &User) -> Result<(), RepositoryError> {
            if let Some(e) = &self.erreur_creation {
                return Err(match e {
                    RepositoryError::CoachNameAlreadyTaken => {
                        RepositoryError::CoachNameAlreadyTaken
                    }
                    RepositoryError::EmailAlreadyTaken => RepositoryError::EmailAlreadyTaken,
                    RepositoryError::Database(m) => RepositoryError::Database(m.clone()),
                });
            }
            *self.dernier_hash.lock().unwrap() = user.password_hash.clone();
            self.crees.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<User>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_legacy_id(&self, _: i32) -> Result<Option<User>, RepositoryError> {
            Ok(None)
        }
        async fn find_by_coach_name(&self, nom: &str) -> Result<Option<User>, RepositoryError> {
            Ok(self.pseudo_pris.then(|| {
                User::new(
                    UserId::new(),
                    CoachName::try_new(nom).unwrap(),
                    None,
                    Email::try_new("occupe@bb.club").unwrap(),
                    "x".into(),
                )
            }))
        }
        async fn update_password_hash(&self, _: &str, _: &str) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    struct FakeJetons {
        crees: AtomicUsize,
        supprimes: AtomicUsize,
    }

    impl FakeJetons {
        fn neuf() -> Self {
            Self {
                crees: AtomicUsize::new(0),
                supprimes: AtomicUsize::new(0),
            }
        }
        fn supprimes(&self) -> usize {
            self.supprimes.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl IResetTokenRepository for FakeJetons {
        async fn find_by_token(&self, _: &str) -> Result<Option<ResetToken>, RepositoryError> {
            Ok(None)
        }
        async fn create(&self, _: &Token, _: &CoachName) -> Result<(), RepositoryError> {
            self.crees.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn delete_by_token(&self, _: &str) -> Result<(), RepositoryError> {
            self.supprimes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct FakeEmail {
        envois: AtomicUsize,
        echoue: bool,
    }

    impl FakeEmail {
        fn qui_marche() -> Self {
            Self {
                envois: AtomicUsize::new(0),
                echoue: false,
            }
        }
        fn en_panne() -> Self {
            Self {
                envois: AtomicUsize::new(0),
                echoue: true,
            }
        }
        fn envoyes(&self) -> usize {
            self.envois.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl IEmailService for FakeEmail {
        async fn send(&self, _: Vec<String>, _: String, _: String) -> Result<(), EmailError> {
            self.envois.fetch_add(1, Ordering::SeqCst);
            if self.echoue {
                Err(EmailError::Network("panne simulée".into()))
            } else {
                Ok(())
            }
        }
    }

    fn cmd(pseudo: &str, email: &str) -> CreateAccountWithoutPasswordCommand {
        CreateAccountWithoutPasswordCommand {
            coach_name: pseudo.into(),
            email: email.into(),
            app_url: "http://exemple.test".into(),
        }
    }

    #[tokio::test]
    async fn creation_nominale_cree_le_compte_envoie_et_emet() {
        let users = FakeUsers::libre();
        let jetons = FakeJetons::neuf();
        let email = FakeEmail::qui_marche();
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let r = execute(
            cmd("NurgleFan", "nurgle@bb.club"),
            &users,
            &jetons,
            &email,
            &bus,
        )
        .await;

        assert!(r.is_ok());
        assert_eq!(users.crees(), 1);
        assert_eq!(email.envoyes(), 1);
        assert_eq!(rx.try_recv().unwrap().event_type, "AccountCreated");
    }

    /// Le compte est créé sans mot de passe utilisable.
    ///
    /// La valeur reprend la convention de Django, d'où vient l'import legacy :
    /// aucune session ne peut s'ouvrir tant que le lien n'a pas été suivi.
    #[tokio::test]
    async fn le_compte_est_cree_sans_mot_de_passe_utilisable() {
        let users = FakeUsers::libre();
        let jetons = FakeJetons::neuf();
        let email = FakeEmail::qui_marche();
        let bus = new_bus();

        execute(
            cmd("NurgleFan", "nurgle@bb.club"),
            &users,
            &jetons,
            &email,
            &bus,
        )
        .await
        .unwrap();

        let hash = users.dernier_hash.lock().unwrap().clone();
        assert_eq!(hash, SANS_MOT_DE_PASSE);
        assert!(
            argon2::PasswordHash::new(&hash).is_err(),
            "le hachage doit être inutilisable par la connexion"
        );
    }

    /// Le pseudo est vérifié **avant** l'envoi.
    ///
    /// Sans ça, une faute de frappe sur un pseudo existant enverrait un lien de
    /// définition de mot de passe à son titulaire actuel, qui n'a rien demandé.
    #[tokio::test]
    async fn un_pseudo_deja_pris_n_envoie_rien_et_ne_cree_rien() {
        let users = FakeUsers::pseudo_pris();
        let jetons = FakeJetons::neuf();
        let email = FakeEmail::qui_marche();
        let bus = new_bus();

        let r = execute(
            cmd("DevCoach", "autre@bb.club"),
            &users,
            &jetons,
            &email,
            &bus,
        )
        .await;

        assert_eq!(r.unwrap_err(), CreateAccountError::PseudoDejaPris);
        assert_eq!(email.envoyes(), 0, "aucun lien ne part vers le titulaire");
        assert_eq!(users.crees(), 0);
    }

    /// Le test qui décide de l'ordre des opérations.
    ///
    /// L'e-mail est l'unique porte d'entrée du compte : s'il ne part pas, aucun
    /// compte ne doit rester, sans quoi le pseudo et l'adresse sont occupés et la
    /// tentative suivante échouerait sur `PseudoDejaPris` sans explication.
    #[tokio::test]
    async fn un_envoi_qui_echoue_ne_laisse_ni_compte_ni_jeton() {
        let users = FakeUsers::libre();
        let jetons = FakeJetons::neuf();
        let email = FakeEmail::en_panne();
        let bus = new_bus();
        let mut rx = bus.subscribe();

        let r = execute(
            cmd("NurgleFan", "nurgle@bb.club"),
            &users,
            &jetons,
            &email,
            &bus,
        )
        .await;

        assert!(matches!(
            r,
            Err(CreateAccountError::EnvoiEmailImpossible(_))
        ));
        assert_eq!(users.crees(), 0, "aucun compte ne doit rester derrière");
        assert_eq!(jetons.supprimes(), 1, "le jeton orphelin est effacé");
        assert!(rx.try_recv().is_err(), "aucun événement");
    }

    /// L'unicité de l'adresse n'est connue qu'à l'insertion — le dépôt n'offre
    /// pas de recherche par e-mail. Le jeton est alors nettoyé.
    #[tokio::test]
    async fn une_adresse_deja_prise_nettoie_le_jeton() {
        let users = FakeUsers::refuse(RepositoryError::EmailAlreadyTaken);
        let jetons = FakeJetons::neuf();
        let email = FakeEmail::qui_marche();
        let bus = new_bus();

        let r = execute(
            cmd("NurgleFan", "occupe@bb.club"),
            &users,
            &jetons,
            &email,
            &bus,
        )
        .await;

        assert_eq!(r.unwrap_err(), CreateAccountError::EmailDejaPris);
        assert_eq!(jetons.supprimes(), 1);
    }

    #[tokio::test]
    async fn un_pseudo_ou_une_adresse_invalide_est_refuse_avant_tout() {
        let bus = new_bus();
        // `!!!` passe désormais le charset — il faut un pseudo réellement
        // invalide, donc un invisible, seul refus qui subsiste côté coach.
        for (pseudo, adresse) in [
            ("Bagouze\u{200B}", "bon@bb.club"),
            ("BonPseudo", "pas-une-adresse"),
        ] {
            let users = FakeUsers::libre();
            let jetons = FakeJetons::neuf();
            let email = FakeEmail::qui_marche();

            let r = execute(cmd(pseudo, adresse), &users, &jetons, &email, &bus).await;

            assert!(r.is_err(), "pour ({pseudo}, {adresse})");
            assert_eq!(email.envoyes(), 0);
            assert_eq!(users.crees(), 0);
        }
    }
}
