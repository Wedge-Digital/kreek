CREATE TABLE comments (
    id         TEXT        NOT NULL PRIMARY KEY,
    legacy_id  INTEGER     UNIQUE,
    article_id TEXT        NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    author_id  TEXT        NOT NULL,
    content    TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX comments_article_id_idx ON comments (article_id);