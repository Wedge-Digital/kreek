//! Les charsets du texte saisi — un seul point de vérité pour l'ensemble de
//! l'application.
//!
//! Onze value objects portaient chacun sa propre expression, et neuf d'entre
//! eux refusaient l'apostrophe. Une compétence nommée « Capitaine d'équipe »
//! s'affichait, s'ajoutait au panier de customisation, et échouait à la
//! validation sur un `UnknownSkill` qui désignait le catalogue plutôt que le
//! nom. Le charset ne se décide plus fichier par fichier.

use regex::Regex;
use std::sync::LazyLock;

/// Le charset de tout texte saisi : compétence, poste, joueur, équipe, roster,
/// espace, saison, journée, tier, compétition.
///
/// Sa base est l'ancien charset de `CompetitionName`, le plus permissif des
/// onze. Il en est donc un sur-ensemble strict : **aucun nom valide hier ne
/// devient invalide**, et rien n'est à migrer.
///
/// S'y ajoutent la typographie française et deux séparateurs :
///
/// - `\p{M}` — les accents décomposés. Un « é » collé depuis macOS peut
///   arriver en « e » + U+0301, forme que `\p{L}` seul refuse. Sans cette
///   classe, on referme le piège d'un côté en le rouvrant de l'autre.
/// - `’ – —` — apostrophe et tirets typographiques, que produit tout
///   traitement de texte par correction automatique. Les formes droites
///   restent acceptées : les deux coexistent, on ne choisit pas pour l'usager.
/// - `« » " “ ” …` — guillemets et points de suspension.
/// - `/ \` — séparateurs, sans danger ici : aucune route ne porte de nom
///   (les chemins ne sont bâtis que sur des identifiants), l'application
///   n'émet aucun fichier, et Askama échappe `< > & " '` à l'affichage.
///
/// Ce qui reste dehors : `| < > { } $ ^` et les caractères de contrôle.
pub static TEXTE_SAISI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^[\p{L}\p{M}\p{N} '’\-–—.,;:!?()\[\]«»"“”…&@#%*+=_°~/\\]+$"#).unwrap()
});

/// Le charset du seul `CoachName`, tenu à l'écart du précédent.
///
/// Ce n'est pas un libellé d'affichage : c'est **l'identifiant de connexion**,
/// celui que `perform_login` cherche par `find_by_coach_name`. Lui ouvrir la
/// ponctuation libre — `@ # % * ! ? / \` — compliquerait la saisie au login
/// sans rien apporter, et rapprocherait dangereusement un pseudonyme d'une
/// adresse électronique.
///
/// Il gagne donc exactement ce que le français exige, et rien de plus :
/// marques combinantes, apostrophes et tirets des deux formes. « Jean-Pierre »
/// et « O’Brien » passent, `Bag@uze` non.
pub static IDENTIFIANT_COACH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}\p{M}\p{N}._ '’\-–—]+$").unwrap());

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

    /// Sans un refus maintenu, ces tests ne prouveraient plus rien.
    #[test]
    fn ce_qui_reste_dehors() {
        assert!(!TEXTE_SAISI.is_match("Ligue|Interdite"));
        assert!(!TEXTE_SAISI.is_match("<script>"));
        assert!(!TEXTE_SAISI.is_match(""));
        assert!(!TEXTE_SAISI.is_match("Ligue\nInterdite"));
        assert!(!TEXTE_SAISI.is_match("Ligue\tInterdite"));
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

    /// Un identifiant de connexion n'est pas un libellé : la ponctuation
    /// libre reste dehors, et `@` en particulier — un pseudonyme ne doit pas
    /// pouvoir prendre l'allure d'une adresse électronique.
    #[test]
    fn l_identifiant_coach_refuse_la_ponctuation_libre() {
        for refuse in ["Bag@uze", "Coach!", "Test#1", "foo/bar", "back\\slash"] {
            assert!(
                !IDENTIFIANT_COACH.is_match(refuse),
                "{refuse} devrait être refusé comme identifiant"
            );
        }
    }

    /// La séparation des deux charsets n'a de sens que si elle se constate.
    #[test]
    fn les_deux_charsets_ne_se_confondent_pas() {
        assert!(TEXTE_SAISI.is_match("Bag@uze"));
        assert!(!IDENTIFIANT_COACH.is_match("Bag@uze"));
    }
}
