//! Autoriser ou interdire les matchs hors calendrier sur une saison (carte 550).
//!
//! # `None` ne vaut pas « saison inconnue », contrairement aux quatre autres
//!
//! `update_visibility_settings_use_case` refuse quand `find_invitations` rend
//! `None` : sans document existant, il n'y a rien à préserver, et en écrire un
//! neuf inventerait des défauts que personne n'a choisis.
//!
//! Ici c'est l'inverse. La colonne `options` est `NULL` sur **toutes** les
//! saisons antérieures à la migration — vingt sur vingt au moment d'écrire ces
//! lignes. Refuser sur `None` rendrait le panneau inutilisable partout, et pour
//! toujours : aucun chemin ne remplit cette colonne avant le premier
//! enregistrement.
//!
//! `None` est donc traité comme « jamais réglée », et le défaut du domaine —
//! qui autorise — sert de base. C'est exact : une saison qui n'a jamais été
//! réglée autorise bel et bien le hors-calendrier.
//!
//! # L'existence de la saison est vérifiée par l'écriture
//!
//! Ne pas lire pour refuser ne veut pas dire ne pas refuser. `save_options`
//! rend `SeasonNotFound` quand son `UPDATE … RETURNING id` ne ramène rien : une
//! saison inconnue est donc rejetée, une ligne plus bas, sans que la lecture ait
//! à s'en charger.
//!
//! # Un seul champ, et pourtant une struct
//!
//! `CompetitionOptions` n'en porte qu'un. Le lire-modifier-réécrire est donc
//! aujourd'hui une formalité — mais c'est le geste qui préservera les suivants
//! le jour où un deuxième réglage s'y ajoutera. L'écrire maintenant coûte trois
//! lignes ; l'oublier alors coûterait un réglage effacé en silence.

use crate::app::competitions::domain::competition_options::{
    AutoriseHorsCalendrier, CompetitionOptions,
};
use crate::app::competitions::domain::season_repository_port::{
    ISeasonRepository, SeasonRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;

#[derive(Debug)]
pub struct UpdateGeneralOptionsCommand {
    pub season_id: SeasonId,
    pub autorise_hors_calendrier: AutoriseHorsCalendrier,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateGeneralOptionsError {
    SeasonNotFound,
    Database(String),
}

impl From<SeasonRepositoryError> for UpdateGeneralOptionsError {
    fn from(e: SeasonRepositoryError) -> Self {
        match e {
            SeasonRepositoryError::SeasonNotFound => Self::SeasonNotFound,
            autre => Self::Database(autre.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: UpdateGeneralOptionsCommand,
    season_repo: &dyn ISeasonRepository,
) -> Result<(), UpdateGeneralOptionsError> {
    let courantes = season_repo
        .find_options(&cmd.season_id)
        .await?
        .unwrap_or_default();

    let nouvelles = CompetitionOptions {
        autorise_hors_calendrier: cmd.autorise_hors_calendrier,
        ..courantes
    };

    season_repo.save_options(&cmd.season_id, &nouvelles).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
    use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
    use crate::app::competitions::domain::competition_rules::CompetitionRules;
    use crate::app::competitions::domain::competition_season::CompetitionSeason;
    use crate::app::competitions::domain::competition_structure::CompetitionStructure;
    use crate::app::competitions::domain::season_repository_port::{SeasonBaseInfo, SeasonFull};
    use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Une doublure qui **retient ce qu'on lui écrit** — la seule façon de
    /// vérifier ce qui part réellement vers la colonne. Les dix-sept autres
    /// méthodes sont copiées telles quelles de la doublure du panneau
    /// Visibilité (règle 5) : ce use case n'en touche aucune.
    struct RepoEspion {
        lues: Option<CompetitionOptions>,
        ecrites: Mutex<Vec<CompetitionOptions>>,
        saison_connue: bool,
    }

    impl RepoEspion {
        fn avec(lues: Option<CompetitionOptions>) -> Self {
            Self {
                lues,
                ecrites: Mutex::new(Vec::new()),
                saison_connue: true,
            }
        }
    }

    #[async_trait]
    impl ISeasonRepository for RepoEspion {
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
            Ok(None)
        }
        async fn save_invitations(
            &self,
            _: &SeasonId,
            _: &CompetitionInvitations,
            _: &CompetitionNotifications,
        ) -> Result<(), SeasonRepositoryError> {
            unimplemented!("hors du périmètre de ce use case")
        }
        async fn save_visibility(
            &self,
            _: &SeasonId,
            _: &CompetitionInvitations,
        ) -> Result<(), SeasonRepositoryError> {
            unimplemented!("hors du périmètre de ce use case")
        }
        async fn espace_interdit_hors_calendrier(
            &self,
            _: &str,
        ) -> Result<bool, SeasonRepositoryError> {
            Ok(false)
        }
        async fn find_options(
            &self,
            _: &SeasonId,
        ) -> Result<Option<CompetitionOptions>, SeasonRepositoryError> {
            Ok(self.lues.clone())
        }
        async fn save_options(
            &self,
            _: &SeasonId,
            options: &CompetitionOptions,
        ) -> Result<(), SeasonRepositoryError> {
            if !self.saison_connue {
                return Err(SeasonRepositoryError::SeasonNotFound);
            }
            self.ecrites.lock().unwrap().push(options.clone());
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

    fn cmd(autorise: bool) -> UpdateGeneralOptionsCommand {
        UpdateGeneralOptionsCommand {
            season_id: SeasonId::new(),
            autorise_hors_calendrier: AutoriseHorsCalendrier(autorise),
        }
    }

    /// **Le cas de toutes les saisons existantes.** Leur colonne est `NULL` ;
    /// refuser sur `None` — ce que fait le panneau Visibilité — rendrait le
    /// réglage inatteignable partout et pour toujours.
    #[tokio::test]
    async fn une_colonne_nulle_n_empeche_pas_d_enregistrer() {
        let repo = RepoEspion::avec(None);

        let r = execute(cmd(false), &repo).await;

        assert_eq!(r, Ok(()));
        let ecrites = repo.ecrites.lock().unwrap();
        assert_eq!(ecrites.len(), 1);
        assert_eq!(
            ecrites[0].autorise_hors_calendrier,
            AutoriseHorsCalendrier(false)
        );
    }

    #[tokio::test]
    async fn interdire_puis_reautoriser_revient_a_l_etat_initial() {
        let repo = RepoEspion::avec(Some(CompetitionOptions {
            autorise_hors_calendrier: AutoriseHorsCalendrier(false),
        }));

        execute(cmd(true), &repo).await.unwrap();

        assert_eq!(
            repo.ecrites.lock().unwrap()[0].autorise_hors_calendrier,
            AutoriseHorsCalendrier(true)
        );
    }

    /// Une saison inconnue est refusée par **l'écriture**, pas par la lecture —
    /// c'est ce qui permet au cas `None` ci-dessus de passer sans ouvrir la
    /// porte aux saisons qui n'existent pas.
    #[tokio::test]
    async fn une_saison_inconnue_est_refusee_a_l_ecriture() {
        let repo = RepoEspion {
            lues: None,
            ecrites: Mutex::new(Vec::new()),
            saison_connue: false,
        };

        let r = execute(cmd(false), &repo).await;

        assert_eq!(r, Err(UpdateGeneralOptionsError::SeasonNotFound));
    }
}
