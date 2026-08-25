//! Masquage d'une adresse électronique destinée à l'affichage.
//!
//! Une liste de coachs sert à **reconnaître** quelqu'un, pas à le contacter :
//! deux homonymes doivent rester distinguables, sans que l'adresse de personne
//! soit livrée à qui ouvre la page.
//!
//! Le masquage se fait **au serveur**, jamais au CSS ni au JS. L'adresse
//! complète voyageait sinon jusqu'au navigateur, lisible dans le source par
//! quiconque sait ouvrir un inspecteur — un masquage qui ne masque que pour
//! l'œil n'en est pas un.
//!
//! Le domaine reste en clair : c'est lui qui distingue deux comptes de même
//! pseudonyme, et il n'identifie personne à lui seul.

/// `bertrand.begouin@gmail.com` → `b•••@gmail.com`
///
/// Une chaîne sans `@` est rendue telle quelle : ce n'est pas une adresse, et
/// la masquer à moitié produirait un libellé trompeur plutôt qu'une protection.
pub fn email_masque(email: &str) -> String {
    let Some((local, domaine)) = email.split_once('@') else {
        return email.to_string();
    };
    match local.chars().next() {
        Some(premiere) => format!("{premiere}•••@{domaine}"),
        // Adresse commençant par `@` — rien à masquer, rien à révéler non plus.
        None => format!("•••@{domaine}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ne_garde_que_la_premiere_lettre_et_le_domaine() {
        assert_eq!(email_masque("bertrand.begouin@gmail.com"), "b•••@gmail.com");
    }

    /// Ce qui justifie de garder le domaine : deux pseudonymes identiques y
    /// restent distinguables.
    #[test]
    fn deux_homonymes_restent_distinguables() {
        assert_ne!(email_masque("marc@gmail.com"), email_masque("marc@free.fr"));
    }

    /// Et ce qu'il ne faut pas attendre du domaine : deux adresses différentes
    /// chez le même fournisseur, de même initiale, deviennent identiques. Le
    /// masquage lève l'ambiguïté le plus souvent, il ne la lève pas toujours.
    #[test]
    fn meme_initiale_et_meme_domaine_se_confondent() {
        assert_eq!(
            email_masque("marc@gmail.com"),
            email_masque("mathilde@gmail.com")
        );
    }

    #[test]
    fn une_chaine_sans_arobase_est_rendue_telle_quelle() {
        assert_eq!(email_masque("pas-une-adresse"), "pas-une-adresse");
    }

    #[test]
    fn une_partie_locale_vide_ne_panique_pas() {
        assert_eq!(email_masque("@exemple.test"), "•••@exemple.test");
    }

    /// La première lettre est un caractère, pas un octet : une adresse
    /// accentuée ne doit pas être tranchée en son milieu.
    #[test]
    fn une_premiere_lettre_accentuee_reste_entiere() {
        assert_eq!(email_masque("élodie@exemple.test"), "é•••@exemple.test");
    }
}
