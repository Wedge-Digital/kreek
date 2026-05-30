CREATE TABLE team_drafts (
    id             TEXT        NOT NULL PRIMARY KEY,
    space_id       TEXT        NOT NULL,
    competition_id TEXT        NOT NULL,
    season_id      TEXT        NOT NULL,
    name           TEXT        NOT NULL,
    coach_id       TEXT        NOT NULL,
    logo           TEXT,
    creation_rules JSONB       NOT NULL,
    status         TEXT        NOT NULL DEFAULT 'draft',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);