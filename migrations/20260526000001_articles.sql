CREATE TABLE articles (
    id         TEXT        NOT NULL PRIMARY KEY,
    space_id   TEXT        NOT NULL,
    author_id  TEXT        NOT NULL,
    title      TEXT        NOT NULL,
    abstract   TEXT        NOT NULL DEFAULT '',
    tags       TEXT[]      NOT NULL DEFAULT '{}',
    image      TEXT,
    content    JSONB       NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX articles_space_id_idx ON articles (space_id);
CREATE INDEX articles_author_id_idx ON articles (author_id);