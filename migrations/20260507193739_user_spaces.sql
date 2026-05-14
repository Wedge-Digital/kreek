-- Add migration script here
CREATE TABLE spaces__user_space
(
    space_id        VARCHAR(26)  NOT NULL,
    coach_id        VARCHAR(26)  NOT NULL,
    coach_name      VARCHAR(100) NOT NULL,
    coach_icon      VARCHAR(255),
    profile         VARCHAR(255) NOT NULL,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE spaces
(
    id                          VARCHAR(26)  NOT NULL,
    space_name                  VARCHAR(100) NOT NULL,
    space_icon_path             VARCHAR(255) NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    CONSTRAINT space_pk         PRIMARY KEY (id),
    CONSTRAINT space_name_uq    UNIQUE (space_name)
);

