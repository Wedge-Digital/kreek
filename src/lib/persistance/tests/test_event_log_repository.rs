use sqlx::PgPool;
use time::OffsetDateTime;
use crate::lib::domain_event::DomainEvent;
use crate::lib::persistance::event_log_repository::{EventLogRepository, IEventLogRepository};

fn make_event(event_type: &str, tags: serde_json::Value, payload: serde_json::Value) -> DomainEvent {
    DomainEvent {
        event_id:    "01JQQQQQQQQQQQQQQQQQQQQB01".into(),
        emitter:     "01JQQQQQQQQQQQQQQQQQQQQ001".into(),
        event_type:  event_type.into(),
        tags,
        payload,
        occurred_at: OffsetDateTime::now_utc(),
    }
}

#[sqlx::test(fixtures("events"))]
async fn find_by_tag_returns_only_matching_events(pool: PgPool) {
    // Given — fixture : 3 events pour team-abc, 1 pour team-xyz
    let repo = EventLogRepository::new(pool);

    // When
    let events = repo.find_by_tag(serde_json::json!({"team_id": "team-abc"})).await.unwrap();

    // Then
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|e| e.tags["team_id"] == "team-abc"));
}

#[sqlx::test(fixtures("events"))]
async fn find_by_tag_returns_events_ordered_by_global_position(pool: PgPool) {
    // Given — fixture : 3 events pour team-abc insérés dans l'ordre
    let repo = EventLogRepository::new(pool);

    // When
    let events = repo.find_by_tag(serde_json::json!({"team_id": "team-abc"})).await.unwrap();

    // Then
    assert_eq!(events.len(), 3);
    assert!(events[0].global_position < events[1].global_position);
    assert!(events[1].global_position < events[2].global_position);
    assert!(events[0].occurred_at < events[1].occurred_at);
    assert!(events[1].occurred_at < events[2].occurred_at);
}

#[sqlx::test(fixtures("events"))]
async fn find_by_tag_returns_empty_for_unknown_tag(pool: PgPool) {
    // Given — fixture chargée, aucun event pour team-inconnu
    let repo = EventLogRepository::new(pool);

    // When
    let events = repo.find_by_tag(serde_json::json!({"team_id": "team-inconnu"})).await.unwrap();

    // Then
    assert!(events.is_empty());
}

#[sqlx::test]
async fn save_persists_event(pool: PgPool) {
    let repo = EventLogRepository::new(pool);
    let event = make_event(
        "TeamDraftCreated",
        serde_json::json!({"team_id": "team-save"}),
        serde_json::json!({"name": "Les Bleus"}),
    );

    repo.save(&event).await.unwrap();

    let found = repo.find_by_tag(serde_json::json!({"team_id": "team-save"})).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].event_type, "TeamDraftCreated");
    assert_eq!(found[0].payload["name"], "Les Bleus");
    assert_eq!(found[0].tags["team_id"], "team-save");
}

#[sqlx::test(fixtures("events"))]
async fn find_by_tag_maps_all_fields_correctly(pool: PgPool) {
    // Given — fixture : premier event de team-abc (position la plus basse)
    let repo = EventLogRepository::new(pool);

    // When
    let events = repo.find_by_tag(serde_json::json!({"team_id": "team-abc"})).await.unwrap();
    let first = &events[0];

    // Then
    assert_eq!(first.event_type, "TeamDraftCreated");
    assert_eq!(first.emitter, "01JQQQQQQQQQQQQQQQQQQQQ001");
    assert_eq!(first.tags["team_id"], "team-abc");
    assert_eq!(first.payload["name"], "Les Bleus");
    assert!(first.global_position > 0);
    assert!(first.recorded_at >= first.occurred_at);
}

#[sqlx::test]
async fn find_by_tag_supports_multi_tag_filter(pool: PgPool) {
    let repo = EventLogRepository::new(pool);
    let event = make_event(
        "TeamDraftCreated",
        serde_json::json!({"team_id": "team-multi", "space_id": "space-1"}),
        serde_json::json!({}),
    );
    repo.save(&event).await.unwrap();

    // When — filtre sur les deux tags
    let found = repo.find_by_tag(serde_json::json!({"team_id": "team-multi", "space_id": "space-1"})).await.unwrap();
    assert_eq!(found.len(), 1);

    // When — filtre partiel : doit aussi matcher
    let found_partial = repo.find_by_tag(serde_json::json!({"team_id": "team-multi"})).await.unwrap();
    assert_eq!(found_partial.len(), 1);

    // When — mauvais space_id : ne doit pas matcher
    let not_found = repo.find_by_tag(serde_json::json!({"team_id": "team-multi", "space_id": "space-99"})).await.unwrap();
    assert!(not_found.is_empty());
}