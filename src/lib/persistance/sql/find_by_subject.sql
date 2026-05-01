SELECT
    id,
    source,
    "type"              AS event_type,
    spec_version,
    time,
    data_schema,
    data_content_type,
    subject,
    data
FROM event_log
WHERE subject = $1
ORDER BY time ASC