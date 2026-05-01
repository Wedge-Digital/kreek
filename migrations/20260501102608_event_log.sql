-- Add migration script here
create table event_log
(
    id                varchar(255)                           not null
        primary key,
    source            varchar(255)                           not null,
    type              varchar(255)                           not null,
    spec_version      varchar(255)                           not null,
    time              timestamp with time zone default now() not null,
    data_schema       varchar(255)                           not null,
    data_content_type varchar(255),
    subject           varchar(255),
    data              text
);

alter table event_log
    owner to root;

create unique index idx_eventlog_id
    on event_log (id);