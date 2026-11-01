//! Le contrat de classes des feuilles imprimées (épic E16).
//!
//! `teams` style le cadre des feuilles (`pages/team-print.css`), mais trois
//! autres BCs le rendent : `players`, `match_report`, `competitions`. Le
//! contrat est écrit dans la spec, et rien ne le tenait : renommer une classe
//! d'un seul côté cassait l'aperçu des trois sans que `check-arch` le voie.
//!
//! Ce test est posé ici, chez l'hôte, parce que c'est lui qui assemble la page
//! et le bundle : il lit la feuille de style et les gabarits tels qu'ils sont
//! livrés, sans qu'aucun BC importe l'autre.

const PAGE_CSS: &str = include_str!("../../../assets/static/css/pages/team-print.css");

/// Les gabarits d'un autre BC qui rendent le cadre des feuilles.
const FEUILLES: &[(&str, &str)] = &[
    (
        "players-print-skills.html",
        include_str!("../../app/players/io/web/templates/players-print-skills.html"),
    ),
    (
        "match-report-print-inducements.html",
        include_str!("../../app/match_report/io/web/templates/match-report-print-inducements.html"),
    ),
    (
        "match-report-print-sheet.html",
        include_str!("../../app/match_report/io/web/templates/match-report-print-sheet.html"),
    ),
];

/// La page, dont le script remplit le numéro de page et la date.
const PAGE_HTML: &str = include_str!("../../app/teams/io/web/templates/teams-print-page.html");

const OPTION_ADVERSAIRE: &str = include_str!(
    "../../app/competitions/io/web/templates/widgets/competitions-print-next-opponent.html"
);

/// Le cadre d'une feuille, stylé par la page : emplacement, cadre réduit,
/// feuille, pied.
const CADRE: &[&str] = &[
    "print-slot",
    "print-frame",
    "print-sheet",
    "print-sheet-foot",
];

/// Les zones que le script de la page remplit : numéro de page, date.
const ZONES_DU_SCRIPT: &[&str] = &["print-sheet-page", "print-sheet-stamp"];

const CHOIX: &[&str] = &[
    "team-print-choice",
    "team-print-choice-main",
    "team-print-choice-hint",
    "team-print-choice--disabled",
];

fn style(classe: &str) -> bool {
    PAGE_CSS.contains(&format!(".{classe} "))
        || PAGE_CSS.contains(&format!(".{classe},"))
        || PAGE_CSS.contains(&format!(".{classe}:"))
        || PAGE_CSS.contains(&format!(".{classe}{{"))
}

#[test]
fn chaque_classe_du_cadre_est_stylee_par_la_page() {
    for classe in CADRE.iter().chain(CHOIX) {
        assert!(style(classe), "team-print.css ne style plus `.{classe}`");
    }
}

#[test]
fn chaque_feuille_d_un_autre_bc_rend_le_cadre() {
    for (gabarit, html) in FEUILLES {
        for classe in CADRE.iter().chain(ZONES_DU_SCRIPT) {
            assert!(
                html.contains(classe),
                "{gabarit} ne rend plus `{classe}`, qu'attendent la page et son script"
            );
        }
    }
}

#[test]
fn le_script_de_la_page_remplit_les_zones_des_feuilles() {
    for classe in ZONES_DU_SCRIPT {
        assert!(
            PAGE_HTML.contains(&format!("'.{classe}'")),
            "le script de teams-print-page.html ne cherche plus `.{classe}`"
        );
    }
}

#[test]
fn l_option_du_prochain_adversaire_rend_les_classes_du_choix() {
    for classe in CHOIX {
        assert!(
            OPTION_ADVERSAIRE.contains(classe),
            "competitions-print-next-opponent.html ne rend plus `{classe}`"
        );
    }
}
