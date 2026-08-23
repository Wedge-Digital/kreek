//! La sous-commande du cron.
//!
//! ```text
//! kreek send-notifications [--date YYYY-MM-DD] [--dry-run]
//! ```
//!
//! `main()` charge configuration, pool **et migrations** avant d'arriver ici :
//! la commande ne peut pas tourner sur un schéma périmé.
//!
//! # `--date` mérite un mot
//!
//! Il permet de viser une date passée, ce que R9 interdit au cron. Ce n'est pas
//! une contradiction : R9 vise le comportement **automatique**, pas une action
//! explicite d'exploitant qui sait ce qu'il fait. La commande le journalise
//! bruyamment quand la date fournie n'est pas celle du jour, pour qu'un
//! `--date` resté dans une crontab finisse par se voir.
//!
//! # Le code de sortie
//!
//! `1` dès qu'un envoi a échoué. C'est ce qui rend R1 observable : une exécution
//! parfaite et une exécution ayant perdu douze e-mails ne doivent pas se
//! ressembler dans les journaux du cron.

use crate::app::competitions::io::repository::match_day_repository::MatchDayRepository;
use crate::app::competitions::io::repository::notification_delivery_repository::NotificationDeliveryRepository;
use crate::app::competitions::io::repository::season_repository::SeasonRepository;
use crate::app::competitions::use_cases::notification_dispatch::DispatchDeps;
use crate::app::competitions::use_cases::send_due_notifications_use_case::{
    self, CronDeps, SendDueNotificationsCommand, SendDueNotificationsReport,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::config::AppConfig;
use sqlx::PgPool;
use std::sync::Arc;
use time::macros::format_description;
use time::OffsetDateTime;

pub async fn execute(
    cfg: &AppConfig,
    pool: &PgPool,
    date: Option<String>,
    dry_run: bool,
    email: Arc<dyn crate::common::services::email::IEmailService>,
) -> Result<SendDueNotificationsReport, String> {
    let aujourdhui = aujourdhui();
    let jour = date.unwrap_or_else(|| aujourdhui.clone());
    if jour != aujourdhui {
        // Bruyant à dessein : un `--date` oublié dans une crontab enverrait
        // chaque nuit les notifications d'un jour figé.
        tracing::warn!(
            demandee = %jour,
            aujourdhui = %aujourdhui,
            "date forcée — ce n'est pas le comportement du cron"
        );
    }
    let today = DateString::try_new(&jour).map_err(|_| format!("date invalide : {jour}"))?;

    // Le câblage est monté ici plutôt que tiré d'`AppState` : le cron n'a pas
    // besoin du routeur, des sessions ni des données de référence, et les
    // construire pour les jeter allongerait le démarrage d'une tâche censée
    // être brève.
    let seasons = SeasonRepository::new(pool.clone());
    let competitions = crate::app::competitions::io::repository::competition_repository::CompetitionRepository::new(pool.clone());
    let match_days = MatchDayRepository::new(pool.clone());
    let journal = NotificationDeliveryRepository::new(pool.clone());
    // Le dépôt d'équipes exige un bus d'évènements ; le cron n'en émet aucun, et
    // personne n'écoute celui-ci. Un bus local évite d'avoir à monter tout le
    // câblage du serveur pour une lecture.
    let (bus, _) = tokio::sync::broadcast::channel(16);
    let teams =
        crate::infrastructure::competitions::team_info_adapter::TeamInfoAdapter::new(Arc::new(
            crate::app::teams::io::repository::team_repository::TeamRepository::new(
                pool.clone(),
                bus,
            ),
        ));
    let members = crate::infrastructure::competitions::space_member_adapter::SpaceMemberAdapter::new(
        Arc::new(crate::app::spaces::io::repository::space_repository::SpaceRepository::new(pool.clone())),
        Arc::new(crate::app::spaces::io::repository::user_cache_repository::SpaceUserCacheRepository::new(pool.clone())),
    );
    let app_url = cfg.app_url();

    let deps = CronDeps {
        seasons: &seasons,
        competitions: &competitions,
        match_days: &match_days,
        journal: &journal,
        dispatch: DispatchDeps {
            teams: &teams,
            members: &members,
            journal: &journal,
            email: email.as_ref(),
            app_url: &app_url,
        },
    };

    Ok(send_due_notifications_use_case::execute(
        SendDueNotificationsCommand { today, dry_run },
        &deps,
    )
    .await)
}

/// La date du jour dans le fuseau du serveur — R10. Le sélecteur par compétition
/// a été retiré à la carte 334 ; il n'y a plus qu'un fuseau, celui d'ici.
fn aujourdhui() -> String {
    OffsetDateTime::now_utc()
        .date()
        .format(format_description!("[year]-[month]-[day]"))
        .unwrap_or_default()
}
