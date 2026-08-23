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
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
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
    /// Pour le nom de l'administrateur, que les deux e-mails d'inscription
    /// nomment en toutes lettres — « **X** t'invite à participer ».
    pub competitions: &'a dyn crate::app::competitions::domain::competition_repository_port::ICompetitionRepository,
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
    let etiquettes = etiquettes(c, &season, deps).await;
    for due in dues {
        let bilan = expedier(&due, &season, cmd, deps, &etiquettes).await;
        rapport.sent += bilan.sent;
        rapport.skipped_already_sent += bilan.skipped_already_sent;
        rapport.failed += bilan.failed;
    }
}

async fn expedier(
    due: &DueNotification,
    season: &SeasonContext<'_>,
    cmd: &SendDueNotificationsCommand,
    deps: &CronDeps<'_>,
    etiquettes: &DispatchLabels,
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
        etiquettes,
        &deps.dispatch,
    )
    .await
}

/// Tout ce que les gabarits nomment. Chaque champ vaut une phrase visible : un
/// `String::new()` ici rend « **** t'invite à participer », ce qui est arrivé
/// entre la carte 340 et sa correction.
async fn etiquettes(
    c: &SeasonCandidate,
    season: &SeasonContext<'_>,
    deps: &CronDeps<'_>,
) -> DispatchLabels {
    let admin = CompetitionId::try_new(&c.competition_id)
        .ok()
        .map(|id| async move { deps.competitions.find_base_info(&id).await.ok().flatten() });
    let admin_name = match admin {
        Some(f) => f
            .await
            .and_then(|b| b.admin_names.first().cloned())
            .unwrap_or_default(),
        None => String::new(),
    };

    DispatchLabels {
        competition_name: c.competition_name.clone(),
        season_name: c.season_name.clone(),
        space_name: c.space_name.clone(),
        admin_name,
        competition_url: format!(
            "{}/app/{}/competitions/{}/{}",
            deps.dispatch.app_url, c.space_id, c.competition_id, c.season_id
        ),
        registration_deadline: season
            .invitations
            .and_then(|i| i.registration_deadline.clone())
            .unwrap_or_default(),
        remaining_slots: places_restantes(season, deps).await,
    }
}

/// « Il reste N places ». Sans plafond déclaré, la phrase n'a pas de valeur à
/// afficher : on rend une chaîne vide **et** le gabarit ne montre alors pas la
/// ligne — c'est mieux que d'annoncer « il reste  places ».
async fn places_restantes(season: &SeasonContext<'_>, deps: &CronDeps<'_>) -> String {
    let Some(max) = season.invitations.and_then(|i| i.max_participants) else {
        return String::new();
    };
    let inscrits = deps
        .dispatch
        .teams
        .find_enrolled_teams(season.season_id)
        .await
        .map(|v| v.len())
        .unwrap_or(0);
    max.saturating_sub(inscrits as u32).to_string()
}
