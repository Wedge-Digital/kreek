//! Noyau d'identité — ce que les BCs `auth` et `spaces` emportent lorsqu'ils
//! sont extraits vers un autre projet. Rien ici ne connaît le Blood Bowl.
pub mod auth_app_events;
pub mod authorization;
pub mod cloudinary;
pub mod coach_definition;
pub mod coach_icon;
pub mod coach_initials;
pub mod coach_name;
pub mod email;
pub mod id_service;
pub mod ids;
pub mod name_vo;
pub mod secret;
pub mod space_definition;
pub mod space_name;
pub mod spaces_app_events;
pub mod sulid;
mod tests;
