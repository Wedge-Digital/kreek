CREATE TABLE users
(
    id            VARCHAR(26)  NOT NULL,
    coach_name    VARCHAR(100) NOT NULL,
    email         VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT users_pk            PRIMARY KEY (id),
    CONSTRAINT users_coach_name_uq UNIQUE (coach_name),
    CONSTRAINT users_email_uq      UNIQUE (email)
);
