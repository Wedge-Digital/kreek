//! Ce que la couche web consomme des BCs (carte 550).
//!
//! **Le premier dossier d'infrastructure qui ne porte pas un nom de BC.** Les
//! huit autres — `competitions`, `match_report`, `news`, `players`, `ranking`,
//! `spaces`, `team_creation`, `teams` — nomment un BC *consommateur*. Ici le
//! consommateur est `src/web/`, le layout, qui n'est pas un BC mais consomme
//! comme un.
//!
//! `web/` a été préféré à `host/` parce qu'il nomme le module qui consomme, et
//! qu'il se lit sans expliquer. Ce ne sera pas la dernière fois que le layout
//! aura besoin d'une donnée de BC.
pub mod hors_calendrier_adapter;
