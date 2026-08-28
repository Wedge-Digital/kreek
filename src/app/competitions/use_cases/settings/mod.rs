//! Les cinq réglages rouverts sur une saison en cours (épic E14).
//!
//! Un use case par panneau, ajouté par sa carte. Le sous-dossier existe pour
//! qu'ils se lisent ensemble — ils partagent une même intention, « modifier une
//! compétition déjà lancée », que le reste de `use_cases/` ne partage pas.

pub mod update_general_settings_use_case;
pub mod update_pools_settings_use_case;
pub mod update_ranking_settings_use_case;
