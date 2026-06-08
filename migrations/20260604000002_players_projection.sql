CREATE TABLE players_projection (
    player_id       TEXT     PRIMARY KEY,
    team_id         TEXT     NOT NULL,
    space_id        TEXT     NOT NULL,
    position_name   TEXT     NOT NULL,
    roster_line_id  TEXT     NOT NULL,
    personal_name   TEXT     NOT NULL DEFAULT '',
    jersey          SMALLINT,
    base_skills     JSONB    NOT NULL DEFAULT '[]',
    acquired_skills JSONB    NOT NULL DEFAULT '[]',
    spp             INT      NOT NULL DEFAULT 0,
    value_kpo       INT      NOT NULL DEFAULT 0,
    version         INT      NOT NULL DEFAULT 1
);

CREATE INDEX players_projection_team_id ON players_projection (team_id);
