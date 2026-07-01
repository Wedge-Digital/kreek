use crate::app::competitions::domain::domain_event::CompetitionsDomainEvent;
use crate::common::initials::initials;
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;

pub fn init(event_bus: &EventBus, pool: PgPool) {
    let mut rx = event_bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(envelope) => {
                    let Ok(event) =
                        serde_json::from_value::<CompetitionsDomainEvent>(envelope.payload.clone())
                    else {
                        continue;
                    };
                    handle_event(event, &pool).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("pairing_projection_listener: lagged by {n}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn handle_event(event: CompetitionsDomainEvent, pool: &PgPool) {
    match event {
        CompetitionsDomainEvent::PairingCreated {
            pairing_id,
            season_id,
            round_id,
            round_name,
            round_position,
            round_date_start,
            round_date_end,
            round_day_type,
            home_team_id,
            home_team_name,
            home_roster_name,
            home_coach_name,
            home_logo_url,
            away_team_id,
            away_team_name,
            away_roster_name,
            away_coach_name,
            away_logo_url,
            ..
        } => {
            insert_projection(
                pool,
                &pairing_id,
                &season_id,
                &round_id,
                &round_name,
                round_position,
                round_date_start.as_deref(),
                round_date_end.as_deref(),
                &round_day_type,
                &home_team_id,
                &home_team_name,
                &home_roster_name,
                &home_coach_name,
                home_logo_url.as_deref(),
                &away_team_id,
                &away_team_name,
                &away_roster_name,
                &away_coach_name,
                away_logo_url.as_deref(),
            )
            .await;
        }
        CompetitionsDomainEvent::PairingDeleted { pairing_id, .. } => {
            delete_projection(pool, &pairing_id).await;
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
async fn insert_projection(
    pool: &PgPool,
    pairing_id: &str,
    season_id: &str,
    round_id: &str,
    round_name: &str,
    round_position: i32,
    round_date_start: Option<&str>,
    round_date_end: Option<&str>,
    round_day_type: &str,
    home_team_id: &str,
    home_team_name: &str,
    home_roster_name: &str,
    home_coach_name: &str,
    home_logo_url: Option<&str>,
    away_team_id: &str,
    away_team_name: &str,
    away_roster_name: &str,
    away_coach_name: &str,
    away_logo_url: Option<&str>,
) {
    let home_initials = initials(home_team_name);
    let away_initials = initials(away_team_name);

    let result = sqlx::query!(
        r#"INSERT INTO competition_match_display_proj (
            pairing_id, season_id, round_id, round_name, round_position,
            round_date_start, round_date_end, round_day_type,
            home_team_id, home_team_name, home_roster_name, home_coach_name, home_logo_url, home_initials,
            away_team_id, away_team_name, away_roster_name, away_coach_name, away_logo_url, away_initials,
            match_status
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8,
            $9, $10, $11, $12, $13, $14,
            $15, $16, $17, $18, $19, $20,
            'upcoming'
        ) ON CONFLICT (pairing_id) DO NOTHING"#,
        pairing_id,
        season_id,
        round_id,
        round_name,
        round_position,
        round_date_start,
        round_date_end,
        round_day_type,
        home_team_id,
        home_team_name,
        home_roster_name,
        home_coach_name,
        home_logo_url,
        home_initials,
        away_team_id,
        away_team_name,
        away_roster_name,
        away_coach_name,
        away_logo_url,
        away_initials,
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::error!("pairing_projection_listener: insert {pairing_id}: {e}");
    }
}

async fn delete_projection(pool: &PgPool, pairing_id: &str) {
    let result = sqlx::query!(
        "DELETE FROM competition_match_display_proj WHERE pairing_id = $1",
        pairing_id
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::error!("pairing_projection_listener: delete {pairing_id}: {e}");
    }
}
