//! Le déclencheur périodique : ce qui est dû aujourd'hui part aujourd'hui.
//!
//! # `today` est une entrée, pas une lecture d'horloge
//!
//! C'est ce qui rend ce use case testable sans attendre le lendemain, et ce qui
//! permet à la CLI d'exposer une date forcée. Un use case qui appelle `now()`
//! lui-même n'est testable qu'en trichant sur l'horloge de la machine.
//!
//! # Le rapport n'est pas décoratif
//!
//! C'est lui que la CLI imprime et dont elle tire son code de sortie :
//! `failed > 0` vaut `exit(1)`, ce qui rend R1 observable dans les journaux du
//! cron. Une exécution parfaitement silencieuse et une exécution ayant perdu
//! douze e-mails ne doivent pas se ressembler.

use crate::app::competitions::domain::notification_delivery::NotificationType;
use crate::app::competitions::domain::notification_schedule::{
    due_today, fenetres, DueNotification,
};
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::io::repository::notification_delivery_repository::{
    NotificationDeliveryRepository, SeasonCandidate,
};
use crate::app::competitions::use_cases::notification_dispatch::{
    dispatch, DispatchDeps, DispatchLabels, DispatchOutcome,
};
use crate::app::competitions::use_cases::notification_recipients::SeasonContext;
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use std::collections::HashMap;

#[derive(Debug)]
pub struct SendDueNotificationsCommand {
    pub today: DateString,
    /// N'écrit rien et n'envoie rien : compte seulement ce qui partirait.
    ///
    /// L'arrêt se fait **avant** la réservation, jamais entre elle et l'envoi :
    /// réserver puis ne rien expédier laisserait des lignes qui bloqueraient le
    /// vrai passage, et R9 interdit de les rejouer.
    pub dry_run: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SendDueNotificationsReport {
    pub seasons_examined: usize,
    pub notifications_due: usize,
    pub sent: usize,
    pub skipped_already_sent: usize,
    pub failed: usize,
}

pub struct CronDeps<'a> {
    pub seasons: &'a dyn ISeasonRepository,
    pub match_days:
        &'a dyn crate::app::competitions::domain::match_day_repository_port::IMatchDayRepository,
    pub journal: &'a NotificationDeliveryRepository,
    pub dispatch: DispatchDeps<'a>,
}

#[tracing::instrument(skip_all, fields(today = %cmd.today.as_ref(), dry_run = cmd.dry_run))]
pub async fn execute(
    cmd: SendDueNotificationsCommand,
    deps: &CronDeps<'_>,
) -> SendDueNotificationsReport {
    let mut rapport = SendDueNotificationsReport::default();

    for candidate in candidates(deps.journal, &cmd.today).await {
        rapport.seasons_examined += 1;
        traiter_saison(&candidate, &cmd, deps, &mut rapport).await;
    }

    tracing::info!(
        seasons = rapport.seasons_examined,
        due = rapport.notifications_due,
        sent = rapport.sent,
        skipped = rapport.skipped_already_sent,
        failed = rapport.failed,
        "cron de notifications terminé"
    );
    rapport
}

/// Les trois requêtes sont bornées par la date ; leurs résultats se recouvrent
/// — une saison peut avoir une journée qui démarre et une autre qui clôt le
/// même jour. On dédoublonne ici plutôt que dans le SQL : `due_today()` est de
/// toute façon appelée une fois par saison, et c'est elle qui décide.
async fn candidates(
    journal: &NotificationDeliveryRepository,
    today: &DateString,
) -> Vec<SeasonCandidate> {
    // Les décalages viennent du domaine, jamais recalculés ici : les requêtes
    // cherchent une journée **à la date donnée**, `due_today()` compare à
    // `today + n`. Les deux dates doivent sortir de la même source, sans quoi le
    // cron ne trouve jamais rien — sans la moindre erreur pour le signaler.
    let Some(f) = fenetres(today) else {
        tracing::error!("date du jour illisible");
        return Vec::new();
    };

    let mut par_id: HashMap<String, SeasonCandidate> = HashMap::new();
    for r in [
        journal
            .seasons_with_round_starting(f.round_eve.as_ref())
            .await,
        journal
            .seasons_with_round_closing(f.round_closing.as_ref())
            .await,
        journal
            .seasons_with_deadline(f.registration_deadline.as_ref())
            .await,
    ] {
        match r {
            Ok(v) => par_id.extend(v.into_iter().map(|c| (c.season_id.clone(), c))),
            Err(e) => tracing::error!("sélection des saisons impossible : {e}"),
        }
    }
    par_id.into_values().collect()
}

async fn traiter_saison(
    c: &SeasonCandidate,
    cmd: &SendDueNotificationsCommand,
    deps: &CronDeps<'_>,
    rapport: &mut SendDueNotificationsReport,
) {
    let (Ok(sid), Ok(space)) = (
        SeasonId::try_new(&c.season_id),
        SpaceId::try_new(&c.space_id),
    ) else {
        rapport.failed += 1;
        return;
    };

    let reglages = deps
        .seasons
        .find_notifications(&sid)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let invitations = deps.seasons.find_invitations(&sid).await.ok().flatten();
    let journees = deps
        .match_days
        .find_by_season(&c.season_id)
        .await
        .unwrap_or_default();

    let dues = due_today(&cmd.today, &journees, invitations.as_ref(), &reglages);
    rapport.notifications_due += dues.len();
    if cmd.dry_run {
        return;
    }

    let season = SeasonContext {
        space_id: &space,
        season_id: &c.season_id,
        invitations: invitations.as_ref(),
    };
    for due in dues {
        let bilan = expedier(&due, c, &season, cmd, deps).await;
        rapport.sent += bilan.sent;
        rapport.skipped_already_sent += bilan.skipped_already_sent;
        rapport.failed += bilan.failed;
    }
}

async fn expedier(
    due: &DueNotification,
    c: &SeasonCandidate,
    season: &SeasonContext<'_>,
    cmd: &SendDueNotificationsCommand,
    deps: &CronDeps<'_>,
) -> DispatchOutcome {
    let (notification, round) = match due {
        DueNotification::RoundEve { round } => (NotificationType::RoundEve, Some(round)),
        DueNotification::RoundClosing { round } => (NotificationType::RoundClosing, Some(round)),
        DueNotification::RegistrationDeadline { .. } => {
            (NotificationType::RegistrationDeadline, None)
        }
    };
    dispatch(
        notification,
        season,
        round,
        &cmd.today,
        &labels(c, season, deps.dispatch.app_url),
        &deps.dispatch,
    )
    .await
}

fn labels(c: &SeasonCandidate, season: &SeasonContext<'_>, app_url: &str) -> DispatchLabels {
    DispatchLabels {
        competition_name: c.competition_name.clone(),
        season_name: c.season_name.clone(),
        space_name: String::new(),
        admin_name: String::new(),
        competition_url: format!("{app_url}/app/{}/competitions", c.space_id),
        registration_deadline: season
            .invitations
            .and_then(|i| i.registration_deadline.clone())
            .unwrap_or_default(),
        remaining_slots: String::new(),
    }
}
