CREATE TABLE competition_seasons (
    id             TEXT        NOT NULL PRIMARY KEY,
    competition_id TEXT        NOT NULL REFERENCES competitions(id) ON DELETE CASCADE,
    name           TEXT        NOT NULL,
    rules          JSONB,
    structure      JSONB,
    invitations    JSONB,
    status         TEXT        NOT NULL DEFAULT 'draft',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE competitions
    DROP COLUMN IF EXISTS rules,
    DROP COLUMN IF EXISTS structure,
    DROP COLUMN IF EXISTS invitations,
    DROP COLUMN IF EXISTS status;