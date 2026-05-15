-- Add migration script here
CREATE TABLE space__user_cache
(
    id                  VARCHAR(26)  NOT NULL,
    coach_name          VARCHAR(100) NOT NULL,
    coach_icon          VARCHAR(255),
    email               VARCHAR(255) NOT NULL,
    created_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT space_user_cache_pk            PRIMARY KEY (id),
    CONSTRAINT space_user_cache_coach_name_uq UNIQUE (coach_name),
    CONSTRAINT space_user_cache_email_uq      UNIQUE (email)
);

