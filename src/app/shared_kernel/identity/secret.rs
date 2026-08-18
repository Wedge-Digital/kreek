//! Une valeur qui n'a rien à faire dans un journal.
//!
//! La carte 347 ajoute `Debug` aux 62 commandes applicatives, parce que la 348
//! journalise la commande reçue par chaque use case. Trois d'entre elles
//! portent un mot de passe, un jeton de réinitialisation ou une adresse
//! e-mail : un `#[derive(Debug)]` posé sans réfléchir mettrait les mots de
//! passe des coachs dans `docker logs`.
//!
//! # Pourquoi un type et pas trois `impl Debug` écrits à la main
//!
//! Un `Debug` manuel protège les champs **qu'on a pensé à masquer, le jour où
//! on l'a écrit**. Ajouter un champ à la struct un an plus tard ne casse rien
//! et ne prévient personne : le nouveau champ est simplement absent du rendu,
//! ou pire, ajouté au `debug_struct` par réflexe. Le type, lui, masque par
//! construction — un secret ne fuit que si quelqu'un écrit `expose()`, ce qui
//! se voit à la relecture et se cherche en un `grep`.
//!
//! # Ce que le type ne fait délibérément pas
//!
//! Pas de `Display`, pas de `Deref`, pas de `AsRef` : chacun rendrait un
//! secret interpolable dans un `{}` ou passable à une fonction attendant un
//! `&str`, c'est-à-dire exactement la fuite qu'on cherche à rendre impossible.
//!
//! Pas de `Serialize` non plus. Un secret sérialisable finirait tôt ou tard
//! dans une charge d'événement, donc dans l'event store — d'où on ne l'efface
//! plus. `Deserialize` existe seul, parce que `PerformLoginCommand` est
//! construite par `Form<…>` depuis le formulaire de connexion.
//!
//! # Ce qu'il couvre
//!
//! Le nom dit « secret », la règle est plus large : **ne doit pas apparaître
//! dans un journal de diagnostic.** Un mot de passe et un jeton sont des
//! identifiants de connexion ; une adresse e-mail est une donnée personnelle.
//! Les trois sortent par la même porte, ils passent par le même type.

use serde::{Deserialize, Deserializer};
use std::fmt;

/// Enveloppe dont le `Debug` ne rend jamais la valeur.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret<T>(T);

/// Ce que voit quiconque journalise un secret.
pub const MASQUE: &str = "[masqué]";

impl<T> Secret<T> {
    pub fn new(valeur: T) -> Self {
        Self(valeur)
    }

    /// Nommée pour être voyante. Un `grep expose()` doit suffire à énumérer
    /// tous les endroits où un secret quitte son enveloppe.
    pub fn expose(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(MASQUE)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Secret<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Secret)
    }
}

impl From<String> for Secret<String> {
    fn from(valeur: String) -> Self {
        Self(valeur)
    }
}

impl From<&str> for Secret<String> {
    fn from(valeur: &str) -> Self {
        Self(valeur.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_debug_ne_rend_jamais_la_valeur() {
        let secret = Secret::new("hunter2-le-mot-de-passe".to_string());

        assert_eq!(format!("{secret:?}"), MASQUE);
    }

    /// Le cas qui compte vraiment : le secret est masqué **à travers** le
    /// `Debug` dérivé de la struct qui le contient, sans que celle-ci ait à
    /// écrire quoi que ce soit.
    #[test]
    fn une_struct_qui_derive_debug_masque_son_secret_sans_rien_faire() {
        #[derive(Debug)]
        struct Commande {
            coach_name: String,
            password: Secret<String>,
        }

        let rendu = format!(
            "{:?}",
            Commande {
                coach_name: "Grimjaw".to_string(),
                password: Secret::new("hunter2-le-mot-de-passe".to_string()),
            }
        );

        assert!(rendu.contains("Grimjaw"), "le diagnostic reste lisible");
        assert!(!rendu.contains("hunter2"), "rendu fuité : {rendu}");
    }

    #[test]
    fn deux_secrets_se_comparent_sans_s_exposer() {
        let saisi = Secret::new("mot-de-passe".to_string());
        let confirme = Secret::new("mot-de-passe".to_string());
        let autre = Secret::new("autre".to_string());

        assert_eq!(saisi, confirme);
        assert_ne!(saisi, autre);
    }
}
