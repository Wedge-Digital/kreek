//! Ce qu'une équipe apporte aux règles qui portent sur les coups de pouce.
//!
//! Deux règles s'en servent, et c'est pourquoi il vit à part :
//!
//! - le **prix** (`inducement_pricing`), réduit par une règle spéciale ;
//! - la **disponibilité** (`inducement_availability`), restreinte par un
//!   `restrictedTo` que le corpus exprime tantôt en roster, tantôt en règle
//!   spéciale, tantôt en membre du staff.
//!
//! Emprunté au catalogue, jamais recopié : c'est le `Team` du corpus réduit à
//! ce que ces deux règles interrogent.

use super::models::Team;

pub struct ProfilRoster<'a> {
    pub roster_id: &'a str,
    pub regles_speciales: &'a [String],
    /// Le staff que cette équipe **peut** engager, et non celui qu'elle a.
    /// L'Apothicaire itinérant n'est offert qu'aux équipes qui ont droit à un
    /// apothicaire ; celles qui n'y ont pas droit ne peuvent pas en louer un
    /// (carte 561).
    pub staff_autorise: &'a [String],
}

impl<'a> ProfilRoster<'a> {
    /// Le profil de ce roster, tel que le catalogue le décrit.
    ///
    /// **Un roster inconnu n'a ni règle spéciale ni staff**, donc paie plein
    /// tarif et n'accède à aucun coup de pouce restreint. C'est la même
    /// prudence que le montant réduit absent : le code ne fabrique aucun
    /// avantage qu'on ne lui a pas donné.
    pub fn depuis(roster_id: &'a str, team: Option<&'a Team>) -> Self {
        Self {
            roster_id,
            regles_speciales: team.map_or(&[], |t| t.special_rules.as_slice()),
            staff_autorise: team.map_or(&[], |t| t.allowed_staff.as_slice()),
        }
    }
}
