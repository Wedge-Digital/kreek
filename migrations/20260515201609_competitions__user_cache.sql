-- Add migration script here
CREATE TABLE competitions__user_cache
(
    id                  VARCHAR(26)  NOT NULL,
    coach_name          VARCHAR(100) NOT NULL,
    coach_icon          VARCHAR(255),
    email               VARCHAR(255) NOT NULL,
    created_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT competitions__user_cache_pk            PRIMARY KEY (id),
    CONSTRAINT competitions__user_cache_coach_name_uq UNIQUE (coach_name),
    CONSTRAINT competitions__user_cache_email_uq      UNIQUE (email)
);

CREATE TABLE  competitions__space_cache
(
    space_id        VARCHAR(26)  NOT NULL,
    space_name      VARCHAR(100) NOT NULL,
    space_icon_path VARCHAR(255) NOT NULL,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT competitions_space_cache_pk      PRIMARY KEY (space_id),
    CONSTRAINT competitions_space_cache_name_uq UNIQUE (space_name)
);

CREATE TABLE  competitions__user_space_cache
(
    space_id        VARCHAR(26)  NOT NULL,
    coach_id        VARCHAR(26)  NOT NULL,
    profile         VARCHAR(255) NOT NULL,
    CONSTRAINT competitions_user_space_cache_pk         PRIMARY KEY (space_id, coach_id),
    CONSTRAINT competitions_user_space_cache_space_fk   FOREIGN KEY (space_id) REFERENCES competitions__space_cache (space_id),
    CONSTRAINT competitions_user_space_cache_coach_fk   FOREIGN KEY (coach_id) REFERENCES competitions__user_cache (id)
);


