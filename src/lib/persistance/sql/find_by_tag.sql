SELECT
    global_position,
    event_id::text AS event_id,
    emitter::text  AS emitter,
    event_type,
    tags,
    payload,
    occurred_at,
    recorded_at
FROM event_log
WHERE tags @> $1
ORDER BY global_position ASC