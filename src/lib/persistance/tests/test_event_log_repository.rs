use sqlx::PgPool;
use crate::lib::persistance::event_log_repository::EventLogRepository;

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