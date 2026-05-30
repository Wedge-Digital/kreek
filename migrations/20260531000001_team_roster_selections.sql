CREATE TABLE team_roster_selections (
    id         TEXT        NOT NULL PRIMARY KEY,
    space_id   TEXT        NOT NULL,
    state      JSONB       NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);