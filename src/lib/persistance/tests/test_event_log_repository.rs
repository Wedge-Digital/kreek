use sqlx::PgPool;
use time::OffsetDateTime;
use crate::lib::persistance::event_log_repository::{EventLogRepository, IEventLogRepository, StoredEvent};

#[sqlx::test(fixtures("events"))]
async fn find_by_subject_returns_only_matching_events(pool: PgPool) {
    // Given — fixture : 3 events pour "team-abc", 1 pour "team-xyz"
    let repo = EventLogRepository::new(pool);

    // When
    let events = repo.find_by_subject("team-abc").await.unwrap();

    // Then
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|e| e.subject.as_deref() == Some("team-abc")));
}

#[sqlx::test(fixtures("events"))]
async fn find_by_subject_returns_events_ordered_oldest_to_newest(pool: PgPool) {
    // Given — fixture : 3 events pour "team-abc" avec timestamps différents
    let repo = EventLogRepository::new(pool);

    // When
    let events = repo.find_by_subject("team-abc").await.unwrap();

    // Then
    assert_eq!(events[0].id, "evt-001"); // 10h00
    assert_eq!(events[1].id, "evt-002"); // 11h00
    assert_eq!(events[2].id, "evt-003"); // 12h00
    assert!(events[0].time < events[1].time);
    assert!(events[1].time < events[2].time);
}

#[sqlx::test(fixtures("events"))]
async fn find_by_subject_returns_empty_for_unknown_subject(pool: PgPool) {
    // Given — fixture chargée, aucun event pour "team-inconnu"
    let repo = EventLogRepository::new(pool);

    // When
    let events = repo.find_by_subject("team-inconnu").await.unwrap();

    // Then
    assert!(events.is_empty());
}

#[sqlx::test]
async fn save_persists_event(pool: PgPool) {
    let repo = EventLogRepository::new(pool);
    let event = StoredEvent {
        id:                "evt-save-01".to_string(),
        source:            "/team-creation".to_string(),
        event_type:        "TeamDraftCreated".to_string(),
        spec_version:      "1.0".to_string(),
        time:              OffsetDateTime::now_utc(),
        data_schema:       "/schemas/team".to_string(),
        data_content_type: Some("application/json".to_string()),
        subject:           Some("team-save".to_string()),
        data:              Some(r#"{"name":"Les Bleus"}"#.to_string()),
    };

    repo.save(&event).await.unwrap();

    let found = repo.find_by_subject("team-save").await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "evt-save-01");
    assert_eq!(found[0].event_type, "TeamDraftCreated");
    assert_eq!(found[0].data.as_deref(), Some(r#"{"name":"Les Bleus"}"#));
}

#[sqlx::test]
async fn save_returns_error_on_duplicate_id(pool: PgPool) {
    let repo = EventLogRepository::new(pool);
    let event = StoredEvent {
        id:                "evt-dup".to_string(),
        source:            "/team-creation".to_string(),
        event_type:        "TeamDraftCreated".to_string(),
        spec_version:      "1.0".to_string(),
        time:              OffsetDateTime::now_utc(),
        data_schema:       "/schemas/team".to_string(),
        data_content_type: None,
        subject:           Some("team-dup".to_string()),
        data:              None,
    };

    repo.save(&event).await.unwrap();
    let result = repo.save(&event).await;
    assert!(result.is_err());
}

#[sqlx::test(fixtures("events"))]
async fn find_by_subject_maps_all_fields_correctly(pool: PgPool) {
    // Given — fixture : evt-001 avec tous les champs renseignés
    let repo = EventLogRepository::new(pool);

    // When
    let events = repo.find_by_subject("team-abc").await.unwrap();
    let first = &events[0];

    // Then
    assert_eq!(first.id, "evt-001");
    assert_eq!(first.source, "/team-creation");
    assert_eq!(first.event_type, "TeamDraftCreated");
    assert_eq!(first.spec_version, "1.0");
    assert_eq!(first.data_schema, "/schemas/team");
    assert_eq!(first.data_content_type.as_deref(), Some("application/json"));
    assert_eq!(first.subject.as_deref(), Some("team-abc"));
    assert!(first.data.is_some());
}