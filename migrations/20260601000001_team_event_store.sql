CREATE TABLE team_event_store (
    id            BIGSERIAL   PRIMARY KEY,
    team_id       TEXT        NOT NULL,
    event_type    TEXT        NOT NULL,
    event_version TEXT        NOT NULL DEFAULT '1.0',
    payload       JSONB       NOT NULL,
    version       BIGINT      NOT NULL,
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX team_event_store_version ON team_event_store (team_id, version);
CREATE INDEX        team_event_store_team_id ON team_event_store (team_id);
