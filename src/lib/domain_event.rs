use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct DomainEventEnvelope {
    pub event_id:    String,
    pub emitter:     String,
    pub event_type:  String,
    pub tags:        serde_json::Value,
    pub payload:     serde_json::Value,
    pub occurred_at: OffsetDateTime,
}
