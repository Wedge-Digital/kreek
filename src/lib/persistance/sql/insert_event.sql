INSERT INTO event_log (id, source, "type", spec_version, time, data_schema, data_content_type, subject, data)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
