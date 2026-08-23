//! Le second déclencheur : l'ouverture des inscriptions.
//!
//! # Pourquoi il n'est pas piloté par le cron
//!
//! R11. Les trois autres notifications se déclenchent sur une **date** comparée
//! à aujourd'hui ; celle-ci se déclenche sur un **fait** — la saison s'ouvre.
//! Il n'y a rien à comparer, et attendre le cron du lendemain ferait arriver
//! l'annonce un jour après l'ouverture.
//!
//! Ce que les deux chemins partagent — et c'est ce qui rend la scission
//! acceptable : le même use case d'expédition, le même journal, le même service
//! de destinataires, les mêmes gabarits. **Seul le déclencheur diffère.**
//!
//! # La date visée
//!
//! Celle de l'ouverture, pas celle de l'envoi — comme partout ailleurs dans
//! cette épic. Les deux coïncident ici, l'ouverture étant justement l'instant
//! du déclenchement, mais la clé reste de la même forme que les trois autres.

use crate::app::competitions::domain::notification_delivery::NotificationType;
use crate::app::competitions::use_cases::notification_dispatch::{
    dispatch, DispatchDeps, DispatchLabels, DispatchOutcome,
};
use crate::app::competitions::use_cases::notification_recipients::SeasonContext;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;

#[derive(Debug)]
pub struct SendRegistrationOpenCommand {
    pub season_id: String,
    pub space_id: String,
    pub competition_name: String,
    pub season_name: String,
    pub opened_on: DateString,
}

#[tracing::instrument(skip_all, fields(season = %cmd.season_id, space = %cmd.space_id))]
pub async fn execute(
    cmd: SendRegistrationOpenCommand,
    invitations: Option<
        &crate::app::competitions::domain::competition_invitations::CompetitionInvitations,
    >,
    deps: &DispatchDeps<'_>,
) -> DispatchOutcome {
    let Ok(space) = crate::app::shared_kernel::identity::ids::SpaceId::try_new(&cmd.space_id)
    else {
        tracing::error!("identifiant d'espace invalide");
        return DispatchOutcome::default();
    };

    let season = SeasonContext {
        space_id: &space,
        season_id: &cmd.season_id,
        invitations,
    };
    let labels = DispatchLabels {
        competition_name: cmd.competition_name.clone(),
        season_name: cmd.season_name.clone(),
        space_name: String::new(),
        admin_name: String::new(),
        competition_url: format!("{}/app/{}/competitions", deps.app_url, cmd.space_id),
        registration_deadline: invitations
            .and_then(|i| i.registration_deadline.clone())
            .unwrap_or_default(),
        remaining_slots: String::new(),
    };

    // Pas de journée : l'ouverture concerne la saison, pas une date de jeu.
    // C'est le cas que l'index protège par `COALESCE(round_id, '')`.
    dispatch(
        NotificationType::RegistrationOpen,
        &season,
        None,
        &cmd.opened_on,
        &labels,
        deps,
    )
    .await
}
