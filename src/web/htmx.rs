//! Ce que l'en-tête `HX-Request` ne dit pas (carte 498).
//!
//! Une route d'onglet répond deux choses selon l'appelant : le fragment de
//! l'onglet quand la barre l'échange, la page entière quand on y arrive de
//! l'extérieur. Le seul en-tête consulté jusqu'ici était `HX-Request` — vrai
//! dans les deux cas.
//!
//! La carte 484 avait déjà buté dessus, côté équipes, et l'avait tranché en
//! supprimant la branche : la route rendait toujours la page, et les appelants
//! internes découpaient au `hx-select`. Ici cette voie coûterait dix routes et
//! six onglets, plus deux paginations qui veulent bel et bien le fragment.
//!
//! `HX-Target` porte l'information manquante : htmx y met l'identifiant de la
//! zone visée. Les onglets ciblent `#tab-content`, les paginations
//! `#calendrier-list` ou `#resultats-list`, et un lien venu d'ailleurs
//! `#app-content` — celui du gabarit d'application.

use axum::http::HeaderMap;

/// L'identifiant de la zone que remplace une navigation d'un écran à l'autre.
/// Il vit dans `web/templates/app-layout.html`.
const CONTENU_APPLICATIF: &str = "app-content";

/// La requête veut-elle **la page entière** plutôt qu'un fragment ?
///
/// Vrai pour une requête ordinaire du navigateur, et pour une requête htmx qui
/// vise le contenu applicatif — c'est-à-dire une navigation.
///
/// **La règle est écrite dans ce sens exprès.** Formulée à l'envers — « fragment
/// si la cible est l'onglet » — elle aurait rendu la page entière aux deux
/// paginations, qui ciblent leur propre liste et n'ont jamais demandé autre
/// chose qu'un fragment. Ici, tout ce qui ne se nomme pas explicitement garde le
/// comportement qu'il avait.
pub fn veut_la_page_entiere(headers: &HeaderMap) -> bool {
    if !headers.contains_key("hx-request") {
        return true;
    }
    headers
        .get("hx-target")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|cible| cible == CONTENU_APPLICATIF)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entetes(paires: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in paires {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    #[test]
    fn une_requete_ordinaire_veut_la_page() {
        assert!(veut_la_page_entiere(&entetes(&[])));
    }

    #[test]
    fn un_lien_vers_le_contenu_applicatif_veut_la_page() {
        assert!(veut_la_page_entiere(&entetes(&[
            ("hx-request", "true"),
            ("hx-target", "app-content"),
        ])));
    }

    /// Le cas de la barre d'onglets — le comportement d'avant, préservé.
    #[test]
    fn un_echange_d_onglet_veut_le_fragment() {
        assert!(!veut_la_page_entiere(&entetes(&[
            ("hx-request", "true"),
            ("hx-target", "tab-content"),
        ])));
    }

    /// **Les deux paginations.** Elles visent leur propre liste ; leur rendre la
    /// page entière l'empilerait sous les résultats déjà affichés.
    #[test]
    fn une_pagination_veut_le_fragment() {
        for cible in ["calendrier-list", "resultats-list"] {
            assert!(
                !veut_la_page_entiere(&entetes(&[("hx-request", "true"), ("hx-target", cible)])),
                "cible {cible}"
            );
        }
    }

    /// Une requête htmx sans cible déclarée garde le fragment : c'est ce
    /// qu'elle obtenait avant, et rien ne dit qu'elle veuille autre chose.
    #[test]
    fn une_requete_htmx_sans_cible_veut_le_fragment() {
        assert!(!veut_la_page_entiere(&entetes(&[("hx-request", "true")])));
    }
}
