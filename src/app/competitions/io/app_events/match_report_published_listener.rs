use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::app::competitions::domain::match_day::Pairing;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, NewPairingProjection,
};
use crate::app::competitions::ports::ITeamInfoPort;
use crate::app::competitions::use_cases::admin::team_enrollment::load_enrolled_teams;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::app_events::match_report_app_events::{
    ActionTypePayload, MatchActionPublishedPayload, MatchReportAppEvent,
    MatchReportPublishedPayload,
};
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::EventId;
use crate::common::services::event_bus::event_bus::EventBus;
use crate::common::services::event_bus::supervision::spawn_listener;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::Instrument;

pub fn init(
    app_event_bus: &EventBus,
    event_bus: EventBus,
    pool: PgPool,
    match_day_repo: Arc<dyn IMatchDayRepository>,
    team_port: Arc<dyn ITeamInfoPort>,
) {
    let mut rx = app_event_bus.subscribe();
    spawn_listener(module_path!(), async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<MatchReportAppEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    let span = tracing::info_span!(
                        "app_event",
                        event = %envelope.event_type,
                        event_id = %envelope.event_id
                    );
                    handle_event(
                        event,
                        &pool,
                        &event_bus,
                        match_day_repo.as_ref(),
                        team_port.as_ref(),
                    )
                    .instrument(span)
                    .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("competitions::match_report_published_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(
    event: MatchReportAppEvent,
    pool: &PgPool,
    event_bus: &EventBus,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
) {
    let MatchReportAppEvent::MatchReportPublished(payload) = event else {
        return;
    };

    let Some(pairing_id) = resolve_pairing_id(&payload, match_day_repo, team_port, event_bus).await
    else {
        return;
    };

    let report_url = AppRoutes::default()
        .match_report
        .recap(&payload.space_id, &payload.match_report_id);
    let home_cas = count_casualties(&payload.home_actions);
    let away_cas = count_casualties(&payload.away_actions);

    let result =
        update_projection(pool, &pairing_id, &payload, home_cas, away_cas, &report_url).await;

    if let Err(e) = result {
        tracing::error!("competitions::match_report_published_listener: update {pairing_id}: {e}");
    }
}

/// Un rapport "manuel" (saisi hors calendrier) n'a pas de pairing — la ligne de
/// projection résultats/calendrier n'existe donc pas encore à ce stade. On crée
/// alors un vrai pairing pour la journée déjà choisie à la création du rapport,
/// exactement comme le ferait "Ajouter un match" : la ligne de projection est
/// insérée en synchrone (même tâche) pour éviter toute course avec l'UPDATE qui
/// suit, et l'event PairingCreated est aussi émis pour la cohérence du système
/// (ex. suivi des équipes déjà affrontées côté génération automatique).
async fn resolve_pairing_id(
    payload: &MatchReportPublishedPayload,
    match_day_repo: &dyn IMatchDayRepository,
    team_port: &dyn ITeamInfoPort,
    event_bus: &EventBus,
) -> Option<String> {
    if let Some(pairing_id) = &payload.pairing_id {
        return Some(pairing_id.clone());
    }

    // Un rapport manuel republié après correction retomberait ici : sans cette
    // recherche, on créerait un second pairing pour le même match, qui
    // apparaîtrait alors deux fois au calendrier.
    match match_day_repo
        .find_pairing_id(
            &payload.round_id,
            &payload.home_team_id,
            &payload.away_team_id,
        )
        .await
    {
        Ok(Some(existing)) => return Some(existing),
        Ok(None) => {}
        Err(e) => {
            tracing::error!("match_report_published_listener: recherche du pairing existant : {e}");
            return None;
        }
    }

    let match_day = match match_day_repo.find_by_id(&payload.round_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            tracing::error!(
                "match_report_published_listener: journée {} introuvable pour le rapport manuel {}",
                payload.round_id,
                payload.match_report_id
            );
            return None;
        }
        Err(e) => {
            tracing::error!(
                "match_report_published_listener: find_by_id journée {}: {e}",
                payload.round_id
            );
            return None;
        }
    };

    let team_display = match load_enrolled_teams(&payload.season_id, team_port).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("match_report_published_listener: load_enrolled_teams: {e}");
            return None;
        }
    };
    let Some(home_info) = team_display.get(&payload.home_team_id) else {
        tracing::error!(
            "match_report_published_listener: équipe domicile {} non enrôlée pour le rapport manuel {}",
            payload.home_team_id, payload.match_report_id
        );
        return None;
    };
    let Some(away_info) = team_display.get(&payload.away_team_id) else {
        tracing::error!(
            "match_report_published_listener: équipe extérieure {} non enrôlée pour le rapport manuel {}",
            payload.away_team_id, payload.match_report_id
        );
        return None;
    };

    let (Ok(home_team_id), Ok(away_team_id)) = (
        TeamId::try_new(&payload.home_team_id),
        TeamId::try_new(&payload.away_team_id),
    ) else {
        tracing::error!(
            "match_report_published_listener: id d'équipe invalide pour {}",
            payload.match_report_id
        );
        return None;
    };
    let pairing = Pairing {
        id: PairingId::new(),
        home_team_id,
        away_team_id,
    };
    let projection = NewPairingProjection {
        season_id: payload.season_id.clone(),
        round_name: match_day.name.to_string(),
        round_position: match_day.position.into_inner(),
        round_date_start: match_day.date_start.as_ref().map(|d| d.to_string()),
        round_date_end: match_day.date_end.as_ref().map(|d| d.to_string()),
        round_day_type: match_day.day_type.as_str().to_string(),
        home_team_name: home_info.team_name.clone(),
        home_roster_name: home_info.roster_name.clone(),
        home_coach_name: home_info.coach_name.clone(),
        home_logo_url: home_info.logo_url.clone(),
        away_team_name: away_info.team_name.clone(),
        away_roster_name: away_info.roster_name.clone(),
        away_coach_name: away_info.coach_name.clone(),
        away_logo_url: away_info.logo_url.clone(),
    };
    if let Err(e) = match_day_repo
        .save_pairing(&payload.round_id, &pairing, &projection)
        .await
    {
        tracing::error!("match_report_published_listener: save_pairing: {e}");
        return None;
    }

    let pairing_id = pairing.id.to_string();

    let _ = event_bus.send(
        CompetitionsDomainEvent::PairingCreated {
            event_id: EventId::new(),
            pairing_id: pairing_id.clone(),
            competition_id: payload.competition_id.clone(),
            season_id: payload.season_id.clone(),
            round_id: payload.round_id.clone(),
            home_team_id: payload.home_team_id.clone(),
            away_team_id: payload.away_team_id.clone(),
            space_id: payload.space_id.clone(),
            home_team_name: home_info.team_name.clone(),
            home_roster_name: home_info.roster_name.clone(),
            home_coach_name: home_info.coach_name.clone(),
            home_logo_url: home_info.logo_url.clone(),
            away_team_name: away_info.team_name.clone(),
            away_roster_name: away_info.roster_name.clone(),
            away_coach_name: away_info.coach_name.clone(),
            away_logo_url: away_info.logo_url.clone(),
            round_name: match_day.name.to_string(),
            round_position: match_day.position.into_inner(),
            round_date_start: match_day.date_start.as_ref().map(|d| d.to_string()),
            round_date_end: match_day.date_end.as_ref().map(|d| d.to_string()),
            round_day_type: match_day.day_type.as_str().to_string(),
        }
        .to_enveloppe(),
    );

    Some(pairing_id)
}

#[allow(clippy::too_many_arguments)]
async fn update_projection(
    pool: &PgPool,
    pairing_id: &str,
    payload: &MatchReportPublishedPayload,
    home_cas: i32,
    away_cas: i32,
    report_url: &str,
) -> Result<(), sqlx::Error> {
    // sqlx est compilé avec la feature `time`, pas `chrono` — conversion
    // nécessaire au point de persistance (l'app event reste en chrono::DateTime<Utc>).
    let published_at = time::OffsetDateTime::from_unix_timestamp_nanos(
        payload.published_at.timestamp_nanos_opt().unwrap_or(0) as i128,
    )
    .expect("date de publication dans la plage représentable par OffsetDateTime");

    sqlx::query!(
        "UPDATE competition_match_display_proj
         SET match_status = 'completed',
             home_score = $2,
             away_score = $3,
             home_casualties = $4,
             away_casualties = $5,
             match_report_url = $6,
             published_at = $7
         WHERE pairing_id = $1",
        pairing_id,
        payload.home_score as i32,
        payload.away_score as i32,
        home_cas,
        away_cas,
        report_url,
        published_at,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Compte les actions Sortie infligées par une équipe (mêmes règles que
/// `MatchReportPreMatch::compute_cas()` côté `match_report` — seule `Sortie`
/// compte comme casualty, `Blesse{..}` en est le résultat côté adverse).
fn count_casualties(actions: &[MatchActionPublishedPayload]) -> i32 {
    actions
        .iter()
        .filter(|a| matches!(a.action, ActionTypePayload::Sortie))
        .count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::app_events::match_report_app_events::PlayerRefPayload;
    use sqlx::Row;

    fn action(action: ActionTypePayload) -> MatchActionPublishedPayload {
        MatchActionPublishedPayload {
            turn: 1,
            player: PlayerRefPayload::Regular {
                player_id: "p1".to_string(),
            },
            action,
        }
    }

    #[test]
    fn count_casualties_empty_returns_zero() {
        assert_eq!(count_casualties(&[]), 0);
    }

    #[test]
    fn count_casualties_counts_sorties_only() {
        let actions = vec![
            action(ActionTypePayload::Sortie),
            action(ActionTypePayload::Sortie),
            action(ActionTypePayload::Touchdown),
        ];
        assert_eq!(count_casualties(&actions), 2);
    }

    #[test]
    fn count_casualties_ignores_blesse() {
        let actions = vec![
            action(ActionTypePayload::Blesse {
                injury: "Commotion".to_string(),
            }),
            action(ActionTypePayload::Blesse {
                injury: "Mort".to_string(),
            }),
        ];
        assert_eq!(count_casualties(&actions), 0);
    }

    use crate::app::competitions::domain::match_day::{
        MatchDay, MatchDayName, MatchDayPosition, MatchDayType,
    };
    use crate::app::competitions::domain::match_day_repository_port::{
        MatchDayRepositoryError, PairingDisplayDto,
    };
    use crate::app::competitions::ports::TeamInfoDto;
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};

    struct FakeMatchDayRepo(MatchDay);
    #[async_trait::async_trait]
    impl IMatchDayRepository for FakeMatchDayRepo {
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
            Ok(Some(self.0.clone()))
        }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn delete_match_day(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &NewPairingProjection,
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, MatchDayRepositoryError> {
            Ok(None)
        }
        async fn delete_pairing(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<(), MatchDayRepositoryError> {
            Ok(())
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            Ok(vec![])
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<
            Vec<crate::app::competitions::domain::match_day_repository_port::LatestResultDto>,
            MatchDayRepositoryError,
        > {
            Ok(vec![])
        }
    }

    struct FakeTeamInfoPort(Vec<TeamInfoDto>);
    #[async_trait::async_trait]
    impl ITeamInfoPort for FakeTeamInfoPort {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(self.0.clone())
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
    }

    fn sample_payload(
        home: &str,
        away: &str,
        pairing_id: Option<String>,
    ) -> MatchReportPublishedPayload {
        MatchReportPublishedPayload {
            match_report_id: "mr1".into(),
            space_id: "sp1".into(),
            competition_id: "c1".into(),
            season_id: "s1".into(),
            round_id: "r1".into(),
            pairing_id,
            published_at: chrono::Utc::now(),
            home_team_id: home.into(),
            away_team_id: away.into(),
            home_score: 1,
            away_score: 0,
            home_gain_kpo: 0,
            home_inducement_spending_kpo: 0,
            away_inducement_spending_kpo: 0,
            away_gain_kpo: 0,
            home_fan_mod: 0,
            away_fan_mod: 0,
            home_actions: vec![],
            away_actions: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
        }
    }

    /// Régression : un rapport manuel (pairing_id: None) n'avait jamais de ligne
    /// dans competition_match_display_proj — donc n'apparaissait jamais dans
    /// "résultats". resolve_pairing_id doit créer un vrai pairing et sa
    /// projection de façon atomique (même transaction, cf. carte 186) avant que
    /// l'UPDATE des scores ne s'exécute. Utilise le vrai `MatchDayRepository`
    /// (pas le fake) car l'écriture atomique vit maintenant dans l'implémentation
    /// réelle de `save_pairing`, avec une vraie contrainte FK sur match_day_id.
    #[sqlx::test]
    async fn resolve_pairing_id_creates_a_real_pairing_and_projection_for_manual_reports(
        pool: PgPool,
    ) {
        let home = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let away = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let match_day = MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 7".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(3).unwrap(),
            pairings: vec![],
        };
        let match_day_repo =
            crate::app::competitions::io::repository::match_day_repository::MatchDayRepository::new(
                pool.clone(),
            );
        match_day_repo
            .save_match_day(&match_day)
            .await
            .expect("insertion de la journée de test");
        let team_port = FakeTeamInfoPort(vec![
            TeamInfoDto {
                team_id: home.into(),
                team_name: "Home".into(),
                coach_id: "coach1".into(),
                coach_name: "C1".into(),
                roster_name: "R1".into(),
                logo_url: None,
            },
            TeamInfoDto {
                team_id: away.into(),
                team_name: "Away".into(),
                coach_id: "coach2".into(),
                coach_name: "C2".into(),
                roster_name: "R2".into(),
                logo_url: None,
            },
        ]);
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut payload = sample_payload(home, away, None);
        payload.round_id = match_day.id.to_string();

        let pairing_id = resolve_pairing_id(&payload, &match_day_repo, &team_port, &event_bus)
            .await
            .expect("un pairing doit être créé pour un rapport manuel");

        let pairing_row =
            sqlx::query("SELECT match_day_id FROM competition_match_day_pairings WHERE id = $1")
                .bind(&pairing_id)
                .fetch_one(&pool)
                .await
                .expect("la ligne de pairing doit exister (même transaction que la projection)");
        assert_eq!(
            pairing_row.get::<String, _>("match_day_id"),
            match_day.id.to_string()
        );

        let row = sqlx::query(
            "SELECT home_team_name, away_team_name, round_name FROM competition_match_display_proj WHERE pairing_id = $1",
        )
        .bind(&pairing_id)
        .fetch_one(&pool)
        .await
        .expect("la ligne de projection doit exister");
        assert_eq!(row.get::<String, _>("home_team_name"), "Home");
        assert_eq!(row.get::<String, _>("away_team_name"), "Away");
        assert_eq!(row.get::<String, _>("round_name"), "Journée 7");
    }

    #[tokio::test]
    async fn resolve_pairing_id_returns_existing_id_unchanged_for_scheduled_reports() {
        let match_day = MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings: vec![],
        };
        let match_day_repo = FakeMatchDayRepo(match_day);
        let team_port = FakeTeamInfoPort(vec![]);
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();
        let payload = sample_payload("home", "away", Some("existing-pairing".into()));

        let pairing_id =
            resolve_pairing_id(&payload, &match_day_repo, &team_port, &event_bus).await;

        assert_eq!(pairing_id, Some("existing-pairing".to_string()));
    }
    /// Régression : republier un rapport **manuel** après correction ne doit pas
    /// recréer un pairing. Sans cette garde, le match apparaissait deux fois au
    /// calendrier dès le second cycle publier / corriger / republier.
    #[sqlx::test]
    async fn republier_un_rapport_manuel_reutilise_le_pairing_existant(pool: PgPool) {
        let home = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let away = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let match_day = MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 9".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(5).unwrap(),
            pairings: vec![],
        };
        let repo =
            crate::app::competitions::io::repository::match_day_repository::MatchDayRepository::new(
                pool.clone(),
            );
        repo.save_match_day(&match_day)
            .await
            .expect("insertion de la journée de test");
        let team_port = FakeTeamInfoPort(vec![
            TeamInfoDto {
                team_id: home.into(),
                team_name: "Home".into(),
                coach_id: "coach1".into(),
                coach_name: "C1".into(),
                roster_name: "R1".into(),
                logo_url: None,
            },
            TeamInfoDto {
                team_id: away.into(),
                team_name: "Away".into(),
                coach_id: "coach2".into(),
                coach_name: "C2".into(),
                roster_name: "R2".into(),
                logo_url: None,
            },
        ]);
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut payload = sample_payload(home, away, None);
        payload.round_id = match_day.id.to_string();

        let premier = resolve_pairing_id(&payload, &repo, &team_port, &event_bus)
            .await
            .unwrap();
        let second = resolve_pairing_id(&payload, &repo, &team_port, &event_bus)
            .await
            .unwrap();

        assert_eq!(
            premier, second,
            "la republication doit réutiliser le pairing existant"
        );

        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM competition_match_day_pairings WHERE match_day_id = $1",
        )
        .bind(match_day.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "un seul pairing, pas deux");
    }

    /// La date réelle de publication doit être stockée en projection — c'est
    /// elle qui permettra de trier des résultats de compétitions différentes
    /// par ordre chronologique (widget "Derniers résultats" de l'accueil).
    #[sqlx::test]
    async fn update_projection_stores_published_at(pool: PgPool) {
        let home = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let away = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let match_day = MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 4".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(1).unwrap(),
            pairings: vec![],
        };
        let repo =
            crate::app::competitions::io::repository::match_day_repository::MatchDayRepository::new(
                pool.clone(),
            );
        repo.save_match_day(&match_day)
            .await
            .expect("insertion de la journée de test");
        let team_port = FakeTeamInfoPort(vec![
            TeamInfoDto {
                team_id: home.into(),
                team_name: "Home".into(),
                coach_id: "coach1".into(),
                coach_name: "C1".into(),
                roster_name: "R1".into(),
                logo_url: None,
            },
            TeamInfoDto {
                team_id: away.into(),
                team_name: "Away".into(),
                coach_id: "coach2".into(),
                coach_name: "C2".into(),
                roster_name: "R2".into(),
                logo_url: None,
            },
        ]);
        let event_bus = crate::common::services::event_bus::event_bus::new_bus();
        let mut payload = sample_payload(home, away, None);
        payload.round_id = match_day.id.to_string();
        // Seconde ronde (pas de sous-seconde) pour comparer sans souci de
        // précision entre chrono (nanos) et la colonne TIMESTAMPTZ (micros).
        payload.published_at = chrono::DateTime::from_timestamp(1_754_000_000, 0).unwrap();

        let pairing_id = resolve_pairing_id(&payload, &repo, &team_port, &event_bus)
            .await
            .expect("un pairing doit être créé pour un rapport manuel");

        update_projection(&pool, &pairing_id, &payload, 0, 0, "http://example/report")
            .await
            .expect("l'update de la projection doit réussir");

        let stored: Option<time::OffsetDateTime> = sqlx::query_scalar(
            "SELECT published_at FROM competition_match_display_proj WHERE pairing_id = $1",
        )
        .bind(&pairing_id)
        .fetch_one(&pool)
        .await
        .expect("la ligne de projection doit exister");

        assert_eq!(
            stored
                .expect("published_at doit être renseigné")
                .unix_timestamp(),
            payload.published_at.timestamp(),
        );
    }
}
