-- Add migration script here
CREATE TABLE event_log (
                        global_position  BIGSERIAL PRIMARY KEY,
                        event_id         VARCHAR(26) NOT NULL,
                        emitter          VARCHAR(26) NOT NULL,
                        event_type       TEXT NOT NULL,
                        tags             JSONB NOT NULL,
                        payload          JSONB NOT NULL,
                        occurred_at      TIMESTAMPTZ NOT NULL,
                        recorded_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_events_tags ON event_log USING GIN (tags);