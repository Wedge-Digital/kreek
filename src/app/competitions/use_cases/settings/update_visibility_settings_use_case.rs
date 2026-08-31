//! Changer le mode d'accès d'une compétition et son mode de validation des
//! inscriptions, sur une saison **en cours**.
//!
//! # Le piège que ce use case contourne
//!
//! `save_invitations` — le chemin qu'emprunte l'étape 4 du magicien — écrit
//! trois choses : les invitations, les notifications, **et
//! `status = 'invitations_configured'`**. Les trois sont justes pendant la
//! création ; aucune ne l'est ici.
//!
//! Le statut d'abord : le reposer ferait régresser la saison sous `ready`,
//! `competition_rules_adapter` ne la dirait plus prête, et la carte 407
//! interdit la création d'équipe sur une saison qui ne l'est pas. Changer un
//! mode d'accès aurait cassé l'inscription de la compétition entière, sans un
//! mot — le défaut exact de la carte 423, à un panneau près.
//!
//! Les notifications ensuite : les écrire oblige à les relire, sous peine de
//! les remettre à leur défaut et d'**éteindre les rappels d'échéance en
//! silence**. `save_visibility` ne touche pas la colonne, ce qui supprime le
//! risque au lieu de le contourner.
//!
//! # Ce qui traverse sans être touché
//!
//! `invited_coaches`, `registration_deadline` et `max_participants` sont relus
//! puis réécrits tels quels. Le panneau ne les édite pas, mais il réécrit le
//! document qui les porte : les omettre les effacerait. `max_participants` en
//! particulier alimente la ligne « il reste N places » des relances.

use crate::app::competitions::domain::competition_invitations::{
    AccessMode, CompetitionInvitations, RequiresValidation,
};
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;

#[derive(Debug)]
pub struct UpdateVisibilitySettingsCommand {
    pub season_id: SeasonId,
    pub access_mode: AccessMode,
    pub requires_validation: RequiresValidation,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateVisibilitySettingsError {
    SeasonNotFound,
    Database(String),
}

impl From<SeasonRepositoryError> for UpdateVisibilitySettingsError {
    fn from(e: SeasonRepositoryError) -> Self {
        Self::Database(e.to_string())
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UpdateVisibilitySettingsCommand,
    season_repo: &dyn ISeasonRepository,
) -> Result<(), UpdateVisibilitySettingsError> {
    // `None` vaut « saison inconnue » **et** « invitations jamais réglées ».
    // Les deux méritent le même refus : sans document existant, il n'y a rien
    // à préserver, et écrire un document neuf inventerait des défauts que
    // personne n'a choisis.
    let courantes = season_repo
        .find_invitations(&cmd.season_id)
        .await?
        .ok_or(UpdateVisibilitySettingsError::SeasonNotFound)?;

    let nouvelles = CompetitionInvitations {
        access_mode: cmd.access_mode,
        requires_validation: cmd.requires_validation,
        invited_coaches: courantes.invited_coaches,
        max_participants: courantes.max_participants,
        registration_deadline: courantes.registration_deadline,
    };

    season_repo
        .save_visibility(&cmd.season_id, &nouvelles)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_invitations::InvitedCoach;
    use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
    use crate::app::competitions::domain::competition_rules::CompetitionRules;
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{SeasonBaseInfo, SeasonFull};
    use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
    use crate::app::shared_kernel::identity::coach_initials::CoachInitials;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::ids::CoachId;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    type Journal = Arc<Mutex<Vec<&'static str>>>;

    struct FakeSeasonRepo {
        invitations: Mutex<Option<CompetitionInvitations>>,
        journal: Journal,
        ecrit: Mutex<Option<CompetitionInvitations>>,
    }

    impl FakeSeasonRepo {
        fn avec(invitations: CompetitionInvitations) -> Self {
            Self {
                invitations: Mutex::new(Some(invitations)),
                journal: Arc::new(Mutex::new(Vec::new())),
                ecrit: Mutex::new(None),
            }
        }
        fn vide() -> Self {
            Self {
                invitations: Mutex::new(None),
                journal: Arc::new(Mutex::new(Vec::new())),
                ecrit: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl ISeasonRepository for FakeSeasonRepo {
        async fn save(&self, _: &CompetitionSeason) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_latest_season_id(
            &self,
            _: &CompetitionId,
        ) -> Result<Option<SeasonId>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_space_id(
            &self,
            _: &SeasonId,
        ) -> Result<Option<String>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_base_info(
            &self,
            _: &SeasonId,
        ) -> Result<Option<SeasonBaseInfo>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn find_rules(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionRules>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_rules(
            &self,
            _: &SeasonId,
            _: &str,
            _: &CompetitionRules,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn save_rules_keep_status(
            &self,
            _: &SeasonId,
            _: &str,
            _: &CompetitionRules,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_structure(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionStructure>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_structure(
            &self,
            _: &SeasonId,
            _: &CompetitionStructure,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn save_structure_and_prune_groups(
            &self,
            _: &SeasonId,
            _: &CompetitionStructure,
            _: &[String],
        ) -> Result<u64, SeasonRepositoryError> {
            unimplemented!("hors du périmètre de ce use case")
        }
        async fn find_invitations(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionInvitations>, SeasonRepositoryError> {
            Ok(self.invitations.lock().unwrap().clone())
        }
        /// **Le chemin interdit.** Il pose `status = 'invitations_configured'`
        /// et réécrit les notifications ; l'appeler ici ferait régresser une
        /// saison en cours. Le journal l'enregistre pour qu'un test puisse
        /// affirmer qu'il n'a pas été emprunté.
        async fn save_invitations(
            &self,
            _: &SeasonId,
            _: &CompetitionInvitations,
            _: &CompetitionNotifications,
        ) -> Result<(), SeasonRepositoryError> {
            self.journal.lock().unwrap().push("save_invitations");
            Ok(())
        }
        async fn save_visibility(
            &self,
            _: &SeasonId,
            invitations: &CompetitionInvitations,
        ) -> Result<(), SeasonRepositoryError> {
            self.journal.lock().unwrap().push("save_visibility");
            *self.ecrit.lock().unwrap() = Some(invitations.clone());
            Ok(())
        }
        async fn find_notifications(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionNotifications>, SeasonRepositoryError> {
            Ok(None)
        }
        async fn save_notifications(
            &self,
            _: &SeasonId,
            _: &CompetitionNotifications,
        ) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn set_ready(&self, _: &SeasonId) -> Result<(), SeasonRepositoryError> {
            Ok(())
        }
        async fn find_full(
            &self,
            _: &SeasonId,
        ) -> Result<Option<SeasonFull>, SeasonRepositoryError> {
            Ok(None)
        }
    }

    // ── Fixtures ─────────────────────────────────────────────────────────────

    fn coach(nom: &str) -> InvitedCoach {
        InvitedCoach {
            id: CoachId::try_new("01M0000000000000000000000A").unwrap(),
            coach_name: CoachName::try_new(nom).unwrap(),
            initials: CoachInitials::try_new("AB").unwrap(),
        }
    }

    /// Une compétition fermée, avec **tout ce que le panneau n'édite pas** :
    /// deux invités, une échéance, un plafond. C'est précisément ce qui doit
    /// survivre.
    fn invitations_garnies() -> CompetitionInvitations {
        CompetitionInvitations {
            access_mode: AccessMode::Invitation,
            requires_validation: RequiresValidation(true),
            invited_coaches: vec![coach("Skarbrand"), coach("Griff")],
            max_participants: Some(12),
            registration_deadline: Some("2026-09-30".to_string()),
        }
    }

    fn commande(mode: AccessMode, validation: bool) -> UpdateVisibilitySettingsCommand {
        UpdateVisibilitySettingsCommand {
            season_id: SeasonId::try_new("01M0000000000000000000000S").unwrap(),
            access_mode: mode,
            requires_validation: RequiresValidation(validation),
        }
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn les_deux_champs_edites_sont_ecrits() {
        let repo = FakeSeasonRepo::avec(invitations_garnies());

        execute(commande(AccessMode::Open, false), &repo)
            .await
            .unwrap();

        let ecrit = repo.ecrit.lock().unwrap().clone().unwrap();
        assert_eq!(ecrit.access_mode, AccessMode::Open);
        assert_eq!(ecrit.requires_validation, RequiresValidation(false));
    }

    #[tokio::test]
    async fn les_coachs_invites_survivent_a_l_ouverture() {
        // Le cas que la carte demande en e2e : passer en « ouvert » ne doit pas
        // perdre la liste, sans quoi refermer la compétition la rendrait vide.
        let repo = FakeSeasonRepo::avec(invitations_garnies());

        execute(commande(AccessMode::Open, true), &repo)
            .await
            .unwrap();

        let ecrit = repo.ecrit.lock().unwrap().clone().unwrap();
        let noms: Vec<String> = ecrit
            .invited_coaches
            .iter()
            .map(|c| c.coach_name.to_string())
            .collect();
        assert_eq!(noms, vec!["Skarbrand".to_string(), "Griff".to_string()]);
    }

    #[tokio::test]
    async fn l_echeance_et_le_plafond_traversent_intacts() {
        // `max_participants` alimente la ligne « il reste N places » des
        // relances : le remettre à zéro l'éteindrait sans rien signaler.
        let repo = FakeSeasonRepo::avec(invitations_garnies());

        execute(commande(AccessMode::Open, false), &repo)
            .await
            .unwrap();

        let ecrit = repo.ecrit.lock().unwrap().clone().unwrap();
        assert_eq!(ecrit.max_participants, Some(12));
        assert_eq!(ecrit.registration_deadline, Some("2026-09-30".to_string()));
    }

    #[tokio::test]
    async fn le_chemin_qui_reecrit_le_statut_n_est_jamais_emprunte() {
        // **Le test qui garde la déviation.** `save_invitations` pose
        // `status = 'invitations_configured'` et réécrit les notifications ;
        // l'emprunter ferait régresser une saison en cours sous `ready`. Le
        // défaut serait invisible : l'enregistrement réussirait.
        let repo = FakeSeasonRepo::avec(invitations_garnies());

        execute(commande(AccessMode::Open, false), &repo)
            .await
            .unwrap();

        let journal = repo.journal.lock().unwrap().clone();
        assert_eq!(journal, vec!["save_visibility"]);
        assert!(
            !journal.contains(&"save_invitations"),
            "le statut et les notifications auraient été réécrits"
        );
    }

    #[tokio::test]
    async fn une_saison_inconnue_est_refusee() {
        let repo = FakeSeasonRepo::vide();

        let r = execute(commande(AccessMode::Open, false), &repo).await;

        assert_eq!(r, Err(UpdateVisibilitySettingsError::SeasonNotFound));
        assert!(
            repo.journal.lock().unwrap().is_empty(),
            "rien ne doit être écrit quand il n'y a rien à préserver"
        );
    }
}
