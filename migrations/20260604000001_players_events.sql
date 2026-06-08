CREATE TABLE players_events (
    id          BIGSERIAL   PRIMARY KEY,
    player_id   TEXT        NOT NULL,
    team_id     TEXT        NOT NULL,
    event_type  TEXT        NOT NULL,
    payload     JSONB       NOT NULL,
    version     INT         NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX players_events_player_version ON players_events (player_id, version);
CREATE INDEX        players_events_player_id      ON players_events (player_id);
CREATE INDEX        players_events_team_id        ON players_events (team_id);
