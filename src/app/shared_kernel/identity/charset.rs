//! Les charsets du texte saisi — un seul point de vérité pour l'ensemble de
//! l'application.
//!
//! # D'une liste blanche à une liste noire
//!
//! Ces expressions ont d'abord énuméré ce qui **était autorisé**. Onze value
//! objects portaient chacun la sienne, et neuf refusaient l'apostrophe : une
//! compétence « Capitaine d'équipe » s'affichait, s'ajoutait au panier, et
//! échouait à la validation sur un `UnknownSkill` qui accusait le catalogue.
//! Le charset unique a fermé ce piège-là ; il ne l'a pas supprimé. Une liste
//! blanche se rouvre au **prochain** caractère oublié, et chacun redevient une
//! carte — `€`, `|`, `<`, les emoji attendaient encore leur tour.
//!
//! Elles énumèrent désormais ce qui est **refusé**, et rien d'autre ne l'est.
//!
//! # Ce n'est pas la défense contre l'injection
//!
//! Celle-ci est l'échappement au rendu, et elle est déjà là : Askama échappe
//! par défaut. Interdire `<` à la saisie ne protégeait de rien qu'on ne
//! protège ailleurs — c'était une contrainte payée par l'usager pour une
//! sécurité déjà acquise.
//!
//! Corollaire : **là où l'échappement ne s'applique pas, le danger revient**.
//! Un nom interpolé dans un `<script>` est rendu en entités HTML que le
//! navigateur n'y décode pas ; il s'affiche corrompu. Ce n'est pas au charset
//! de compenser, c'est au gabarit de passer par un attribut de données.

use regex::Regex;
use std::sync::LazyLock;

/// Ce qu'aucun texte saisi ne peut contenir.
///
/// | Refusé | Pourquoi |
/// |---|---|
/// | `\p{Cc}` — contrôles C0/C1, dont `\n` `\r` `\t` | un nom est **une ligne** : un saut casse les journaux, les en-têtes et les exports |
/// | `\p{Zl}` `\p{Zp}` — U+2028/2029 | mêmes conséquences, et hors de `Cc` |
/// | U+202A–202E, U+2066–2069 | overrides bidirectionnels : un nom s'afficherait à l'envers de ce qu'il contient — *Trojan Source* appliqué à l'interface |
///
/// Passent donc `< > | € « » — / \ !`, les emoji et les alphabets non latins.
///
/// U+200D (ZWJ) reste **autorisé** : le refuser casserait les séquences emoji
/// composées et plusieurs écritures indiennes. C'est un choix, pas un oubli.
///
/// Ce que cette règle ne couvre pas : une chaîne faite d'espaces insécables
/// est un nom invisible et valide. `sanitize(trim)` ne les retire pas. Le cas
/// ne s'est jamais présenté ; il demanderait une normalisation, pas un
/// caractère de plus dans une liste.
pub static TEXTE_SAISI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[^\p{Cc}\p{Zl}\p{Zp}\x{202A}-\x{202E}\x{2066}-\x{2069}]+$").unwrap()
});

/// Le charset du seul `CoachName`, tenu à l'écart du précédent.
///
/// Ce n'est pas un libellé d'affichage : c'est **l'identifiant de connexion**,
/// celui que `perform_login` cherche par `find_by_coach_name`, et dont
/// l'unicité est portée par `users_coach_name_lower_uq` — insensible à la
/// casse, mais **octet par octet** pour tout le reste.
///
/// D'où le refus de `\p{Cf}` entier, et non des seuls overrides : un espace de
/// largeur nulle inséré dans « Bagouze » produirait un second compte,
/// visuellement identique au premier, dans la liste des coachs comme dans les
/// résultats de match. Le prix est de ne pas pouvoir porter d'emoji composé
/// dans un pseudonyme — acceptable pour un identifiant.
///
/// Ce refus subsume les deux plages de `TEXTE_SAISI` : elles sont elles-mêmes
/// des `Cf`. Elles ne sont donc pas répétées ici.
///
/// **Ce que ça ne couvre pas** : les homoglyphes entre alphabets — un `а`
/// cyrillique passe pour un `a` latin. Il faudrait une normalisation et une
/// table de confusables ; c'est une carte à part, le jour où le besoin se
/// présente.
pub static IDENTIFIANT_COACH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[^\p{Cc}\p{Cf}\p{Zl}\p{Zp}]+$").unwrap());

#[cfg(test)]
mod tests {
    use super::*;

    /// nutype ne vérifie une expression à la compilation que si elle est
    /// littérale. Passée par une constante, elle n'est compilée qu'au premier
    /// usage : une faute de syntaxe se manifesterait par un `panic` en
    /// production. Chacun de ces tests la touche, ce qui referme le trou.
    #[test]
    fn le_cas_qui_a_motive_ce_charset() {
        assert!(TEXTE_SAISI.is_match("Capitaine d'équipe"));
    }

    #[test]
    fn les_deux_apostrophes_passent() {
        assert!(TEXTE_SAISI.is_match("Coup d'épaule"));
        assert!(TEXTE_SAISI.is_match("Coup d’épaule"));
    }

    /// Un « é » collé depuis macOS peut arriver décomposé. Sans `\p{M}`, la
    /// chaîne est refusée alors qu'elle s'affiche à l'identique — le genre de
    /// rejet qu'aucun usager ne peut comprendre ni corriger.
    #[test]
    fn un_accent_decompose_passe_comme_un_accent_compose() {
        assert!(TEXTE_SAISI.is_match("équipe"));
        assert!(TEXTE_SAISI.is_match("e\u{301}quipe"));
    }

    #[test]
    fn les_tirets_typographiques_passent() {
        assert!(TEXTE_SAISI.is_match("Championnat Étoilé — Saison 5"));
        assert!(TEXTE_SAISI.is_match("Ligue 2025–2026"));
        assert!(TEXTE_SAISI.is_match("Jean-Pierre"));
    }

    #[test]
    fn les_separateurs_passent() {
        assert!(TEXTE_SAISI.is_match("Saison 2025/2026"));
        assert!(TEXTE_SAISI.is_match("Poule A\\B"));
    }

    #[test]
    fn les_guillemets_et_points_de_suspension_passent() {
        assert!(TEXTE_SAISI.is_match("La « Grande » Ligue"));
        assert!(TEXTE_SAISI.is_match("La \"Grande\" Ligue"));
        assert!(TEXTE_SAISI.is_match("Et ainsi de suite…"));
    }

    /// Le charset est un sur-ensemble de l'ancien charset des compétitions :
    /// ce qu'il acceptait déjà doit continuer de passer, sans quoi des noms
    /// en base deviendraient illisibles au chargement.
    #[test]
    fn ce_que_les_competitions_acceptaient_deja_passe_encore() {
        assert!(TEXTE_SAISI.is_match("Ligue (Hiver) 2026 - Phase 1"));
        assert!(TEXTE_SAISI.is_match("Coupe #1 @ 100% ~ [A+B*C=D]"));
    }

    /// Ce que la bascule ouvre, et qui distingue vraiment la liste noire de
    /// l'ancienne liste blanche : chacune de ces valeurs était refusée hier.
    #[test]
    fn ce_que_la_liste_noire_ouvre() {
        for accepte in [
            "Équipe <Étoilée>",
            "Ligue €uro",
            "Journée|1",
            "Team 🏈",
            "Famille 👨‍👩‍👧",
            "Coût $100",
            "Poule {A}",
            "Puissance 2^10",
        ] {
            assert!(
                TEXTE_SAISI.is_match(accepte),
                "« {accepte} » devrait passer : rien n'y casse quoi que ce soit"
            );
        }
    }

    /// La liste noire n'est une règle que si elle refuse. Chacune de ces
    /// valeurs casse quelque chose d'identifiable — pas « ça fait peur ».
    #[test]
    fn ce_qui_reste_dehors() {
        for refuse in [
            "Ligne1\nLigne2",
            "Tab\there",
            "retour\rchariot",
            "evil\u{202E}nom",
            "isolat\u{2066}bidi",
            "sep\u{2028}ligne",
            "para\u{2029}graphe",
            "",
        ] {
            assert!(
                !TEXTE_SAISI.is_match(refuse),
                "{refuse:?} devrait être refusé"
            );
        }
    }

    /// La séquence emoji composée repose sur U+200D. Le refuser fermerait
    /// aussi plusieurs écritures indiennes — d'où son maintien, qui est un
    /// choix et mérite d'être tenu par un test.
    #[test]
    fn le_liant_de_largeur_nulle_reste_admis() {
        assert!(TEXTE_SAISI.is_match("a\u{200D}b"));
    }

    #[test]
    fn l_identifiant_coach_admet_ce_que_le_francais_exige() {
        assert!(IDENTIFIANT_COACH.is_match("Jean-Pierre"));
        assert!(IDENTIFIANT_COACH.is_match("O’Brien"));
        assert!(IDENTIFIANT_COACH.is_match("D'Artagnan"));
        assert!(IDENTIFIANT_COACH.is_match("Bâgouze"));
        assert!(IDENTIFIANT_COACH.is_match("e\u{301}quipe"));
        assert!(IDENTIFIANT_COACH.is_match("Bagouze_2"));
        assert!(IDENTIFIANT_COACH.is_match("Dark Nagash"));
    }

    /// Ce qui distingue l'identifiant du libellé : **les invisibles**, et eux
    /// seuls. Un espace de largeur nulle dans un pseudonyme produirait un
    /// second compte impossible à distinguer du premier à l'œil.
    #[test]
    fn l_identifiant_coach_refuse_les_invisibles() {
        for refuse in ["Bagouze\u{200B}", "Bag\u{200D}ouze", "Bagouze\u{FEFF}"] {
            assert!(
                !IDENTIFIANT_COACH.is_match(refuse),
                "{refuse:?} devrait être refusé : deux comptes identiques à l'œil"
            );
        }
    }

    /// La séparation des deux charsets n'a de sens que si elle se constate.
    /// Elle ne tient plus qu'à une chose — les invisibles.
    #[test]
    fn les_deux_charsets_ne_se_confondent_que_sur_les_invisibles() {
        assert!(TEXTE_SAISI.is_match("Famille 👨‍👩‍👧"));
        assert!(!IDENTIFIANT_COACH.is_match("Famille 👨‍👩‍👧"));

        // Tout le reste passe des deux côtés, désormais.
        assert!(TEXTE_SAISI.is_match("Bag@uze"));
        assert!(IDENTIFIANT_COACH.is_match("Bag@uze"));
    }
}
