-- Add migration script here
create table team_creation__draft_team
(
    entity_id  CHAR(26)                 not null primary key,
    created_by CHAR(26)                 not null,
    coached_by CHAR(26)                 not null,
    serialized jsonb                    not null,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

create table team_creation__coach
(
    entity_id  CHAR(26) not null primary key,
    name VARCHAR(255) not null,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);



