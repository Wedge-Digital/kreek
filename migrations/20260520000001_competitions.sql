CREATE TABLE competitions
(
    id         VARCHAR(26)  NOT NULL,
    space_id   VARCHAR(26)  NOT NULL,
    name       VARCHAR(100) NOT NULL,
    logo       VARCHAR(255) NOT NULL,
    status     VARCHAR(50)  NOT NULL DEFAULT 'draft',
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    CONSTRAINT competitions_pk           PRIMARY KEY (id),
    CONSTRAINT competitions_space_fk     FOREIGN KEY (space_id) REFERENCES competitions__space_cache (space_id),
    CONSTRAINT competitions_name_space_uq UNIQUE (space_id, name)
);

CREATE TABLE competitions_members
(
    competition_id VARCHAR(26) NOT NULL,
    coach_id       VARCHAR(26) NOT NULL,
    competition_profile VARCHAR(255) NOT NULL,
    CONSTRAINT competitions_admins_pk             PRIMARY KEY (competition_id, coach_id),
    CONSTRAINT competitions_admins_competition_fk FOREIGN KEY (competition_id) REFERENCES competitions (id),
    CONSTRAINT competitions_admins_coach_fk       FOREIGN KEY (coach_id) REFERENCES competitions__user_cache (id)
);