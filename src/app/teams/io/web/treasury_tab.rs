//! L'onglet « Trésorerie » de la fiche d'équipe (carte 436).
//!
//! Il n'y a **pas de handler ici** : la 434 a posé l'aiguillage dans
//! `team_detail.rs`, où une seule route sert les deux chemins — le fragment
//! sous `HX-Request`, la page entière sinon. Ce module ne fournit que le
//! contenu de l'onglet, et le traduit en réponse HTTP quand il échoue.

use crate::app::teams::io::web::builders::build_treasury_vm;
use crate::app::teams::io::web::treasury_view_models::{RowKind, TreasuryVm};
use crate::app::teams::use_cases::treasury_statement_service::{
    self, TreasuryStatementError as Erreur,
};
use crate::state::AppState;
use askama::Template;
use axum::http::StatusCode;

#[derive(Template)]
#[template(path = "teams-treasury-tab.html")]
pub struct TreasuryTabTemplate {
    pub vm: TreasuryVm,
}

/// Le contenu de l'onglet, rendu.
///
/// # Pourquoi `500` et non `422`
///
/// Un `422` dirait au coach qu'il a mal fait quelque chose ; il n'y est pour
/// rien. Une dotation absente ou un motif illisible décrivent une base qui ne
/// devrait pas exister, et la seule action utile est qu'ils apparaissent au
/// journal avec leur `rid` — **le motif compris**, puisque c'est lui qu'on
/// cherchera.
pub async fn rendre_onglet(team_id: &str, state: &AppState) -> Result<String, StatusCode> {
    let statement = treasury_statement_service::build_statement(
        team_id,
        state.teams.team_repository.as_ref(),
        state.teams.squad_port.as_ref(),
        state.teams.match_context_port.as_ref(),
    )
    .await
    .map_err(|e| journaliser(team_id, e))?;

    TreasuryTabTemplate {
        vm: build_treasury_vm(&statement),
    }
    .render()
    .map_err(|e| {
        tracing::error!("teams treasury tab render {team_id}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn journaliser(team_id: &str, e: Erreur) -> StatusCode {
    match e {
        Erreur::MissingOpeningEntry => {
            tracing::error!("treasury {team_id}: grand livre sans dotation de départ")
        }
        Erreur::UnknownReason(motif) => {
            tracing::error!("treasury {team_id}: motif de mouvement illisible « {motif} »")
        }
        Erreur::Repository(detail) => {
            tracing::error!("treasury {team_id}: lecture du grand livre — {detail}")
        }
    }
    StatusCode::INTERNAL_SERVER_ERROR
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::io::web::treasury_view_models::{GroupVm, MovementRowVm, SummaryVm};

    fn ligne(kind: RowKind, is_credit: bool, detail: Option<&str>) -> MovementRowVm {
        MovementRowVm {
            date_label: "12 août".into(),
            icon: "🏁",
            label: "Dotation de départ".into(),
            detail: detail.map(str::to_string),
            amount_label: "+510 kPo".into(),
            balance_label: "510 kPo".into(),
            kind,
            is_credit,
        }
    }

    fn rendu(groups: Vec<GroupVm>, is_opening_only: bool) -> String {
        let movement_count = groups.iter().map(|g| g.rows.len() as u32).sum();
        TreasuryTabTemplate {
            vm: TreasuryVm {
                summary: SummaryVm {
                    opening_kpo: 510,
                    credited_kpo: 60,
                    debited_kpo: 90,
                    balance_kpo: 480,
                },
                groups,
                is_opening_only,
                movement_count,
            },
        }
        .render()
        .unwrap()
    }

    fn groupe(heading: Option<&str>, rows: Vec<MovementRowVm>) -> GroupVm {
        GroupVm {
            heading: heading.map(str::to_string),
            rows,
        }
    }

    /// **La correspondance nature → classe ne vit que dans le gabarit.**
    ///
    /// `nature()` est testé côté builder, mais rien ne dit qu'une `Correction`
    /// obtient bien `tr-row--fix` : deux bras de `{% match %}` intervertis
    /// rendraient un HTML parfaitement valide, et le défaut ne se verrait qu'à
    /// l'écran, en couleur.
    #[test]
    fn chaque_nature_de_ligne_porte_ses_classes() {
        let cas = [
            (RowKind::Opening, true, "tr-row--origin", "tr-icon\">"),
            (RowKind::Credit, true, "", "tr-icon tr-icon--credit"),
            (RowKind::Debit, false, "", "tr-icon tr-icon--debit"),
            (
                RowKind::Correction,
                false,
                "tr-row--fix",
                "tr-icon tr-icon--fix",
            ),
        ];

        for (kind, is_credit, classe_de_ligne, classe_d_icone) in cas {
            let html = rendu(
                vec![groupe(None, vec![ligne(kind, is_credit, None)])],
                false,
            );

            assert!(html.contains(classe_d_icone), "{kind:?} : icône — {html}");
            if classe_de_ligne.is_empty() {
                assert!(
                    !html.contains("tr-row--origin") && !html.contains("tr-row--fix"),
                    "{kind:?} ne doit porter aucune classe de ligne — {html}"
                );
            } else {
                assert!(html.contains(classe_de_ligne), "{kind:?} : ligne — {html}");
            }
        }
    }

    /// Le badge « Correction » n'appartient qu'aux lignes qui défont un
    /// mouvement — c'est lui qui distingue une recette annulée d'une dépense.
    #[test]
    fn seule_une_correction_porte_le_badge() {
        let correction = rendu(
            vec![groupe(None, vec![ligne(RowKind::Correction, false, None)])],
            false,
        );
        let debit = rendu(
            vec![groupe(None, vec![ligne(RowKind::Debit, false, None)])],
            false,
        );

        assert!(correction.contains("tr-badge-fix"));
        assert!(!debit.contains("tr-badge-fix"), "{debit}");
    }

    /// La couleur du montant suit le sens du mouvement, pas la nature : une
    /// correction rend de l'argent ou en reprend, et les deux se lisent.
    #[test]
    fn la_couleur_du_montant_suit_le_sens_du_mouvement() {
        let rendu_credit = rendu(
            vec![groupe(None, vec![ligne(RowKind::Correction, true, None)])],
            false,
        );
        let rendu_debit = rendu(
            vec![groupe(None, vec![ligne(RowKind::Correction, false, None)])],
            false,
        );

        assert!(
            rendu_credit.contains(r#"tr-amount credit"#),
            "{rendu_credit}"
        );
        assert!(rendu_debit.contains(r#"tr-amount debit"#), "{rendu_debit}");
    }

    /// Un titre de période devient un séparateur ; son absence n'en produit
    /// aucun — sans quoi l'ouverture serait précédée d'une ligne vide.
    #[test]
    fn seul_un_groupe_titre_pose_un_separateur() {
        let avec = rendu(
            vec![groupe(
                Some("Journée 1 — contre Les Trolls du Bief"),
                vec![ligne(RowKind::Credit, true, None)],
            )],
            false,
        );
        let sans = rendu(
            vec![groupe(None, vec![ligne(RowKind::Credit, true, None)])],
            false,
        );

        assert!(avec.contains("tr-row--sep"));
        assert!(avec.contains("Journée 1 — contre Les Trolls du Bief"));
        assert!(!sans.contains("tr-row--sep"), "{sans}");
    }

    /// Un détail absent ne laisse **aucun** `<div>` : vide, il prendrait sa
    /// marge et décalerait le libellé.
    #[test]
    fn un_detail_absent_ne_laisse_pas_de_bloc() {
        let avec = rendu(
            vec![groupe(
                None,
                vec![ligne(RowKind::Credit, true, Some("Gwenn, Passeuse — n° 7"))],
            )],
            false,
        );
        let sans = rendu(
            vec![groupe(None, vec![ligne(RowKind::Credit, true, None)])],
            false,
        );

        assert!(avec.contains("tr-detail"));
        assert!(avec.contains("Gwenn, Passeuse — n° 7"));
        assert!(!sans.contains("tr-detail"), "{sans}");
    }

    /// **L'état vide garde son bandeau.** Une équipe neuve a bien 510 kPo, et
    /// masquer le solde avec le tableau serait dire qu'elle n'a rien.
    #[test]
    fn l_etat_vide_remplace_le_tableau_mais_garde_le_bandeau() {
        let vide = rendu(
            vec![groupe(None, vec![ligne(RowKind::Opening, true, None)])],
            true,
        );

        assert!(vide.contains("Aucun mouvement pour l'instant"));
        assert!(!vide.contains("tr-table"), "aucun tableau : {vide}");
        assert!(vide.contains("tr-summary"), "le bandeau reste : {vide}");
        assert!(vide.contains("tr-balance-value"));
    }

    /// Contre-épreuve du précédent : dès qu'il y a des mouvements, c'est le
    /// tableau qui s'affiche, et le bloc vide disparaît.
    #[test]
    fn le_tableau_remplace_l_etat_vide() {
        let plein = rendu(
            vec![groupe(
                None,
                vec![
                    ligne(RowKind::Opening, true, None),
                    ligne(RowKind::Debit, false, None),
                ],
            )],
            false,
        );

        assert!(plein.contains("tr-table"));
        assert!(!plein.contains("Aucun mouvement pour l'instant"), "{plein}");
        assert!(plein.contains("2 mouvements"));
    }
}
