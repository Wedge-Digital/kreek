//! Les réglages généraux d'une saison — aujourd'hui un seul (carte 550).
//!
//! # Pas d'agrégat, et pas encore de raison d'en faire un
//!
//! Un booléen indépendant, aucune combinaison interdite, aucun invariant à
//! garder. Lui donner des méthodes de commande serait inventer une règle qui
//! n'existe pas — même nature que `CompetitionNotifications`,
//! `CompetitionInvitations` et `CompetitionStructure`, qui n'en ont pas non
//! plus.
//!
//! # Le défaut vaut `true`, et ce n'est pas un choix de confort
//!
//! La colonne `options` est `NULL` sur toutes les saisons antérieures à la
//! migration. Un champ absent se rend donc par le défaut serde, qui autorise —
//! c'est exactement le comportement d'aujourd'hui, où le hors-calendrier est
//! toujours possible et sans réglage.
//!
//! Interdire reste ainsi un geste délibéré. Le défaut inverse aurait fermé la
//! saisie manuelle dans toutes les ligues au déploiement, sans que personne
//! l'ait demandé.

use serde::{Deserialize, Serialize};

/// Vrai quand un coach peut saisir un match en choisissant lui-même les deux
/// équipes et la journée, sans passer par une rencontre du calendrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AutoriseHorsCalendrier(pub bool);

fn autorise() -> AutoriseHorsCalendrier {
    AutoriseHorsCalendrier(true)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompetitionOptions {
    #[serde(default = "autorise")]
    pub autorise_hors_calendrier: AutoriseHorsCalendrier,
}

impl Default for CompetitionOptions {
    fn default() -> Self {
        Self {
            autorise_hors_calendrier: autorise(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le cas qui compte.** Toutes les saisons existantes portent `NULL` dans
    /// la colonne : c'est ce chemin-là qui décide de leur comportement après
    /// déploiement, et il doit autoriser.
    #[test]
    fn un_document_vide_autorise_le_hors_calendrier() {
        let options: CompetitionOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(
            options.autorise_hors_calendrier,
            AutoriseHorsCalendrier(true)
        );
    }

    #[test]
    fn le_defaut_de_la_struct_autorise_aussi() {
        assert_eq!(
            CompetitionOptions::default().autorise_hors_calendrier,
            AutoriseHorsCalendrier(true)
        );
    }

    /// Un `false` écrit est relu `false` : sans ce test, un défaut qui
    /// écraserait la valeur lue passerait inaperçu — le premier test seul ne le
    /// verrait pas.
    #[test]
    fn une_interdiction_enregistree_se_relit() {
        let options: CompetitionOptions =
            serde_json::from_str(r#"{"autorise_hors_calendrier": false}"#).unwrap();
        assert_eq!(
            options.autorise_hors_calendrier,
            AutoriseHorsCalendrier(false)
        );
    }

    /// `#[serde(transparent)]` : le document porte un booléen nu, pas un objet
    /// enveloppant. Le figer ici évite qu'un changement de dérive rende
    /// illisibles les colonnes déjà écrites.
    #[test]
    fn la_forme_serialisee_est_un_booleen_nu() {
        let json = serde_json::to_string(&CompetitionOptions {
            autorise_hors_calendrier: AutoriseHorsCalendrier(false),
        })
        .unwrap();
        assert_eq!(json, r#"{"autorise_hors_calendrier":false}"#);
    }
}
