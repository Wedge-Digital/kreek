INSERT INTO event_log (event_id, emitter, event_type, tags, payload, occurred_at)
VALUES ($1::text::uuid, $2::text::uuid, $3, $4, $5, $6)