//! Des lignes du dépôt aux blocs de l'accordéon.

use crate::app::ranking::io::web::manual_points::view_models::{ManualPointVm, ManualPointsTeamVm};
use crate::app::ranking::ports::{EnrolledTeamInfo, ICoachDataPort, ManualPointRow};
use crate::app::routes::AppRoutes;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Les identifiants du chemin, portés ensemble parce qu'ils voyagent ensemble.
pub struct Chemin<'a> {
    pub space_id: &'a str,
    pub competition_id: &'a str,
    pub season_id: &'a str,
}

/// Signe explicite, y compris pour un positif : la colonne se lit comme une
/// contribution au total, pas comme une quantité.
pub fn signe(valeur: i32) -> String {
    match valeur < 0 {
        true => format!("−{}", valeur.abs()),
        false => format!("+{valeur}"),
    }
}

/// La classe qui colore le signe, du même côté que lui : les deux dérivent de
/// la valeur, les séparer les ferait diverger.
fn classe(valeur: i32) -> &'static str {
    match valeur < 0 {
        true => "minus",
        false => "plus",
    }
}

/// « 4 lignes · 3 équipes concernées ».
pub fn libelle_releve(lignes: usize, equipes: usize) -> String {
    let l = match lignes {
        1 => "1 ligne".to_string(),
        n => format!("{n} lignes"),
    };
    let e = match equipes {
        1 => "1 équipe concernée".to_string(),
        n => format!("{n} équipes concernées"),
    };
    format!("{l} · {e}")
}

/// **`BTreeMap` et non `HashMap`** : l'ordre des blocs doit être stable d'un
/// affichage à l'autre. Un ordre de hachage ferait sauter les équipes de place
/// à chaque rechargement, sans qu'aucune donnée n'ait changé.
/// Les noms des commissaires d'un relevé, en une passe.
///
/// **Les identifiants distincts, pas les lignes.** Une journée de forfaits, ce
/// sont vingt attributions par le même commissaire : résoudre ligne à ligne
/// ferait vingt allers-retours pour un seul nom. `BTreeSet` dédoublonne et
/// ordonne, ce qui rend aussi la suite d'appels reproductible en test.
///
/// Un identifiant que le port ne résout pas **n'entre pas** dans la table : son
/// absence est ce que `to_vm` lit pour rendre `None`.
pub async fn resolve_coach_names(
    coach_data: &dyn ICoachDataPort,
    rows: &[ManualPointRow],
) -> HashMap<String, String> {
    let distincts: BTreeSet<&str> = rows.iter().map(|r| r.awarded_by.as_str()).collect();
    let mut noms = HashMap::new();
    for id in distincts {
        if let Some(nom) = coach_data.find_coach_name(id).await {
            noms.insert(id.to_string(), nom);
        }
    }
    noms
}

pub fn build_teams(
    rows: Vec<ManualPointRow>,
    teams: &[EnrolledTeamInfo],
    chemin: &Chemin<'_>,
    noms: &HashMap<String, String>,
) -> Vec<ManualPointsTeamVm> {
    let mut par_equipe: BTreeMap<String, Vec<ManualPointRow>> = BTreeMap::new();
    for row in rows {
        par_equipe.entry(row.team_id.clone()).or_default().push(row);
    }

    par_equipe
        .into_iter()
        .map(|(team_id, lignes)| {
            let total: i32 = lignes.iter().map(|l| l.points).sum();
            let compte = lignes.len();
            ManualPointsTeamVm {
                team_name: nom_de(&team_id, teams),
                team_id,
                total: signe(total),
                total_class: classe(total),
                line_count: compte,
                line_label: libelle_lignes(compte),
                lines: lignes.into_iter().map(|l| to_vm(l, chemin, noms)).collect(),
            }
        })
        .collect()
}

fn to_vm(
    row: ManualPointRow,
    chemin: &Chemin<'_>,
    noms: &HashMap<String, String>,
) -> ManualPointVm {
    ManualPointVm {
        delete_url: AppRoutes::default().ranking.manual_point(
            chemin.space_id,
            chemin.competition_id,
            chemin.season_id,
            row.id,
        ),
        id: row.id,
        points: signe(row.points),
        points_class: classe(row.points),
        // Un motif vide en base vaut « pas de motif » : le champ est facultatif,
        // et une chaîne vide afficherait une cellule vide plutôt qu'un tiret.
        reason: row.reason.filter(|r| !r.trim().is_empty()),
        awarded_by: noms.get(&row.awarded_by).cloned(),
        awarded_at: format_date(row.awarded_at),
    }
}

fn libelle_lignes(n: usize) -> String {
    match n {
        1 => "1 ligne".to_string(),
        _ => format!("{n} lignes"),
    }
}

/// À défaut d'inscription retrouvée, l'identifiant tient lieu de nom — mieux
/// qu'une ligne vide, et le problème se voit. Même parti pris que
/// `resolve_team_name` du classement.
fn nom_de(team_id: &str, teams: &[EnrolledTeamInfo]) -> String {
    teams
        .iter()
        .find(|t| t.team_id == team_id)
        .map(|t| t.team_name.clone())
        .unwrap_or_else(|| team_id.to_string())
}

/// « 19 août » — jour et mois, comme la maquette. L'heure n'apprend rien sur une
/// décision de commissaire, et l'année encombrerait : un relevé de saison ne
/// couvre pas deux ans.
fn format_date(t: time::OffsetDateTime) -> String {
    const MOIS: [&str; 12] = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    format!("{} {}", t.day(), MOIS[usize::from(u8::from(t.month())) - 1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn ligne(id: i64, team: &str, points: i32, motif: Option<&str>) -> ManualPointRow {
        ManualPointRow {
            id,
            team_id: team.to_string(),
            points,
            reason: motif.map(str::to_string),
            awarded_by: "DevCoach".to_string(),
            awarded_at: datetime!(2026-08-19 14:30 UTC),
        }
    }

    fn chemin() -> Chemin<'static> {
        Chemin {
            space_id: "E1",
            competition_id: "C1",
            season_id: "S1",
        }
    }

    fn equipes() -> Vec<EnrolledTeamInfo> {
        vec![
            EnrolledTeamInfo {
                team_id: "T1".into(),
                team_name: "Trolls du Bief".into(),
            },
            EnrolledTeamInfo {
                team_id: "T2".into(),
                team_name: "Griffons d'Argent".into(),
            },
        ]
    }

    #[test]
    fn le_groupement_par_equipe_somme_les_lignes() {
        let blocs = build_teams(
            vec![
                ligne(1, "T1", 3, Some("forfait")),
                ligne(2, "T2", -3, Some("sanction")),
                ligne(3, "T1", 2, None),
            ],
            &equipes(),
            &chemin(),
            &HashMap::new(),
        );

        assert_eq!(blocs.len(), 2);
        let t1 = blocs.iter().find(|b| b.team_id == "T1").unwrap();
        assert_eq!(t1.total, "+5", "3 + 2");
        assert_eq!(t1.total_class, "plus");
        let t2 = blocs.iter().find(|b| b.team_id == "T2").unwrap();
        assert_eq!(t2.total, "−3");
        assert_eq!(t2.total_class, "minus");
    }

    #[test]
    fn le_pluriel_des_lignes_suit_le_nombre() {
        let blocs = build_teams(
            vec![
                ligne(1, "T1", 3, None),
                ligne(2, "T1", 1, None),
                ligne(3, "T2", 1, None),
            ],
            &equipes(),
            &chemin(),
            &HashMap::new(),
        );

        let t1 = blocs.iter().find(|b| b.team_id == "T1").unwrap();
        let t2 = blocs.iter().find(|b| b.team_id == "T2").unwrap();
        assert_eq!(t1.line_label, "2 lignes");
        assert_eq!(t2.line_label, "1 ligne");
    }

    #[test]
    fn le_libelle_du_releve_accorde_ses_deux_nombres() {
        assert_eq!(libelle_releve(1, 1), "1 ligne · 1 équipe concernée");
        assert_eq!(libelle_releve(4, 3), "4 lignes · 3 équipes concernées");
    }

    /// **L'ordre des blocs doit être stable.** Un ordre de hachage ferait sauter
    /// les équipes de place à chaque rechargement, sans qu'aucune donnée n'ait
    /// changé — le genre de bougé qu'on attribue à un bug ailleurs.
    #[test]
    fn l_ordre_des_blocs_ne_depend_pas_de_l_ordre_des_lignes() {
        let a = build_teams(
            vec![ligne(1, "T2", 1, None), ligne(2, "T1", 1, None)],
            &equipes(),
            &chemin(),
            &HashMap::new(),
        );
        let b = build_teams(
            vec![ligne(2, "T1", 1, None), ligne(1, "T2", 1, None)],
            &equipes(),
            &chemin(),
            &HashMap::new(),
        );

        let ids = |v: &[ManualPointsTeamVm]| -> Vec<String> {
            v.iter().map(|t| t.team_id.clone()).collect()
        };
        assert_eq!(ids(&a), ids(&b));
        assert_eq!(ids(&a), vec!["T1".to_string(), "T2".to_string()]);
    }

    /// Un motif vide en base vaut « pas de motif » : le champ est facultatif, et
    /// une chaîne vide afficherait une cellule vide plutôt qu'un tiret.
    #[test]
    fn un_motif_blanc_vaut_une_absence_de_motif() {
        let blocs = build_teams(
            vec![ligne(1, "T1", 3, Some("   ")), ligne(2, "T1", 1, None)],
            &equipes(),
            &chemin(),
            &HashMap::new(),
        );

        assert!(blocs[0].lines.iter().all(|l| l.reason.is_none()));
    }

    /// L'identifiant tient lieu de nom à défaut d'inscription retrouvée — mieux
    /// qu'une ligne vide, et le problème se voit. Même parti pris que le
    /// classement.
    #[test]
    fn une_equipe_inconnue_s_affiche_par_son_identifiant() {
        let blocs = build_teams(
            vec![ligne(1, "T_INCONNUE", 3, None)],
            &equipes(),
            &chemin(),
            &HashMap::new(),
        );

        assert_eq!(blocs[0].team_name, "T_INCONNUE");
    }

    #[test]
    fn l_url_de_retrait_porte_les_quatre_identifiants() {
        let blocs = build_teams(
            vec![ligne(42, "T1", 3, None)],
            &equipes(),
            &chemin(),
            &HashMap::new(),
        );

        let url = &blocs[0].lines[0].delete_url;
        assert!(url.contains("/E1/"), "{url}");
        assert!(url.contains("/C1/"), "{url}");
        assert!(url.contains("/S1/"), "{url}");
        assert!(url.ends_with("/42"), "{url}");
    }

    #[test]
    fn la_date_se_rend_en_jour_et_mois() {
        let blocs = build_teams(
            vec![ligne(1, "T1", 3, None)],
            &equipes(),
            &chemin(),
            &HashMap::new(),
        );

        assert_eq!(blocs[0].lines[0].awarded_at, "19 août");
    }

    // ── Le nom du commissaire (carte 548) ─────────────────────────────────────

    /// Un port bouchon qui **compte ses appels** : c'est le seul moyen de
    /// prouver le dédoublonnage, qui ne se voit pas dans le résultat.
    struct PortCompteur {
        connus: HashMap<String, String>,
        appels: std::sync::Mutex<Vec<String>>,
    }

    impl PortCompteur {
        fn avec(paires: &[(&str, &str)]) -> Self {
            Self {
                connus: paires
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                appels: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl ICoachDataPort for PortCompteur {
        async fn find_coach_name(&self, coach_id: &str) -> Option<String> {
            self.appels.lock().unwrap().push(coach_id.to_string());
            self.connus.get(coach_id).cloned()
        }
    }

    fn ligne_de(id: i64, team: &str, par: &str) -> ManualPointRow {
        ManualPointRow {
            id,
            team_id: team.to_string(),
            points: 3,
            reason: None,
            awarded_by: par.to_string(),
            awarded_at: datetime!(2026-08-19 14:30 UTC),
        }
    }

    #[tokio::test]
    async fn le_nom_du_commissaire_remplace_son_identifiant() {
        let port = PortCompteur::avec(&[("01K4F2M8QX", "Bagouze")]);
        let lignes = vec![ligne_de(1, "T1", "01K4F2M8QX")];

        let noms = resolve_coach_names(&port, &lignes).await;
        let blocs = build_teams(lignes, &equipes(), &chemin(), &noms);

        assert_eq!(blocs[0].lines[0].awarded_by.as_deref(), Some("Bagouze"));
    }

    /// Un compte supprimé ne se résout plus. Le VM rend `None`, et le gabarit un
    /// tiret — pas l'identifiant, qui est le défaut qu'on corrige, ni un libellé
    /// inventé, qui ferait passer une absence pour une donnée.
    #[tokio::test]
    async fn un_commissaire_introuvable_ne_laisse_pas_son_identifiant() {
        let port = PortCompteur::avec(&[]);
        let lignes = vec![ligne_de(1, "T1", "01K4F2M8QX")];

        let noms = resolve_coach_names(&port, &lignes).await;
        let blocs = build_teams(lignes, &equipes(), &chemin(), &noms);

        assert_eq!(blocs[0].lines[0].awarded_by, None);
    }

    /// **Le cas qui motive la fonction.** Une journée de forfaits, ce sont vingt
    /// attributions par le même commissaire : sans dédoublonnage, la page ferait
    /// vingt requêtes pour un seul nom.
    #[tokio::test]
    async fn un_identifiant_repete_n_est_interroge_qu_une_fois() {
        let port = PortCompteur::avec(&[("A", "Bagouze"), ("B", "Ork")]);
        let lignes = vec![
            ligne_de(1, "T1", "A"),
            ligne_de(2, "T1", "A"),
            ligne_de(3, "T2", "B"),
            ligne_de(4, "T2", "A"),
        ];

        let noms = resolve_coach_names(&port, &lignes).await;

        let appels = port.appels.lock().unwrap().clone();
        assert_eq!(appels, vec!["A".to_string(), "B".to_string()], "{appels:?}");
        assert_eq!(noms.len(), 2);
    }
}
