-- Add migration script here
create table auth__lost_login_token
(
    token       varchar(255)                not null primary key,
    created_at  timestamp(6) with time zone not null,
    coach_name    varchar(255)                not null
);
