//! L'onglet Paramètres : rouvrir cinq réglages sur une saison en cours.
//!
//! `settings_tab` assemble ; chaque panneau vit dans son propre module, ajouté
//! par sa carte (421 à 425). Le dossier existe dès la coquille pour que les cinq
//! arrivent à côté de leur assemblage plutôt que dispersés dans `admin/`.

pub mod general_panel;
pub mod settings_tab;
