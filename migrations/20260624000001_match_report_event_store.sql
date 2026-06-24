CREATE TABLE match_report_event_store (
    id                BIGSERIAL   PRIMARY KEY,
    match_report_id   TEXT        NOT NULL,
    event_type        TEXT        NOT NULL,
    event_version     TEXT        NOT NULL DEFAULT '1.0',
    payload           JSONB       NOT NULL,
    version           BIGINT      NOT NULL,
    occurred_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX match_report_es_version ON match_report_event_store (match_report_id, version);
CREATE INDEX match_report_es_id ON match_report_event_store (match_report_id);

CREATE TABLE match_report_projection (
    match_report_id   TEXT        PRIMARY KEY,
    space_id          TEXT        NOT NULL,
    competition_id    TEXT        NOT NULL,
    season_id         TEXT        NOT NULL,
    round_id          TEXT        NOT NULL,
    home_team_id      TEXT        NOT NULL,
    away_team_id      TEXT        NOT NULL,
    created_by        TEXT        NOT NULL,
    origin            TEXT        NOT NULL,
    phase             TEXT        NOT NULL,
    version           BIGINT      NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX match_report_proj_space  ON match_report_projection (space_id);
CREATE INDEX match_report_proj_season ON match_report_projection (season_id);
CREATE INDEX match_report_proj_coach  ON match_report_projection (created_by, space_id);
