//! La construction du relevé de trésorerie affichable (carte 436).
//!
//! **Tout le formatage de l'écran est ici, et nulle part ailleurs** : les dates,
//! les montants, les huit libellés de motif, les emojis, les titres de période.
//! Ni le service, qui rend des types, ni le gabarit, qui n'a aucune logique.
//!
//! Un `builders.rs` plutôt qu'un `from_domain()` sur le view model : le relevé
//! descend d'un service qui interroge un port, et le `CLAUDE.md` réserve le
//! constructeur co-localisé aux vues qui se construisent depuis le seul domaine.

use crate::app::teams::domain::treasury::{MovementDirection, MovementReason};
use crate::app::teams::io::web::treasury_view_models::{
    GroupVm, MovementRowVm, RowKind, SummaryVm, TreasuryVm,
};
use crate::app::teams::use_cases::treasury_statement_service::{
    LineMatchContext, StatementLine, TreasuryStatement,
};

pub fn build_treasury_vm(statement: &TreasuryStatement) -> TreasuryVm {
    TreasuryVm {
        summary: resume(statement),
        groups: periodes(&statement.lines),
        // La dotation seule : il y a bien une ligne, mais rien ne s'est encore
        // passé. C'est l'état d'une équipe qui vient d'être créée.
        is_opening_only: statement.lines.len() <= 1,
        movement_count: statement.lines.len() as u32,
    }
}

// ── Le bandeau ────────────────────────────────────────────────────────────────

/// `earned` du service **compte la dotation** — c'est un crédit comme un autre
/// dans le grand livre. L'équation du bandeau la sort pour lui donner sa propre
/// colonne, sans quoi elle serait comptée deux fois.
///
/// Les quatre termes sont positifs par construction : le grand livre n'écrit que
/// des montants effectifs, planchés à zéro. Le `max(0)` garde la conversion
/// honnête plutôt que de la faire déborder en silence.
fn resume(statement: &TreasuryStatement) -> SummaryVm {
    SummaryVm {
        opening_kpo: positif(statement.opening),
        credited_kpo: positif(statement.earned - statement.opening),
        debited_kpo: positif(statement.spent),
        balance_kpo: positif(statement.balance),
    }
}

fn positif(v: i32) -> u32 {
    v.max(0) as u32
}

// ── Les périodes ──────────────────────────────────────────────────────────────

/// **Un titre ouvre une période ; il n'étiquette pas les lignes qui suivent.**
///
/// Une ligne qui porte un contexte de match différent de celui en cours ouvre
/// une période ; toutes les autres rejoignent celle qui est ouverte. C'est ce
/// que fait un relevé de compte, et c'est la seule chose que les données
/// permettent : la recette de match — la ligne la plus fréquente — ne porte
/// aucun identifiant de rapport, donc aucun contexte.
///
/// Limite assumée : deux matchs consécutifs contre le même adversaire dans la
/// même journée fusionneraient en une seule période. Sans occurrence dans une
/// compétition à journées, et cosmétique si elle se produisait.
fn periodes(lines: &[StatementLine]) -> Vec<GroupVm> {
    let debuts = debuts_de_periode(lines);
    let mut groupes = vec![ouverture()];
    let mut prochain = 0;

    for (i, line) in lines.iter().enumerate() {
        if let Some((_, titre)) = debuts.get(prochain).filter(|(debut, _)| *debut == i) {
            groupes.push(periode(titre));
            prochain += 1;
        }
        if let Some(g) = groupes.last_mut() {
            g.rows.push(ligne(line));
        }
    }

    // L'ouverture est vide si la toute première ligne ouvrait déjà une période —
    // impossible aujourd'hui, la dotation ouvrant tout relevé.
    groupes.retain(|g| !g.rows.is_empty());
    groupes
}

/// Où commence chaque période, et sous quel titre.
fn debuts_de_periode(lines: &[StatementLine]) -> Vec<(usize, String)> {
    let mut debuts: Vec<(usize, String)> = Vec::new();
    let mut courante: Option<(String, String)> = None;

    for (i, line) in lines.iter().enumerate() {
        let Some(ctx) = line.match_context.as_ref() else {
            continue;
        };
        let cle = (ctx.round_name.clone(), ctx.opponent_name.clone());
        if courante.as_ref() == Some(&cle) {
            continue;
        }
        courante = Some(cle);
        debuts.push((remonter_la_sequence(lines, i), titre_de_periode(ctx)));
    }
    debuts
}

/// **Le titre doit s'ouvrir avant la recette de son propre match.**
///
/// La recette d'après-match ne porte aucun identifiant de rapport — seuls les
/// coups de pouce et les corrections en portent — et le grand livre l'écrit
/// *avant* eux : mesuré le 2026-08-30, 110 équipes sur 110, sans exception. Un
/// découpage qui n'ouvrirait la période qu'à la première ligne identifiable
/// rangerait donc **chaque** recette au-dessus du titre de son match, où elle se
/// lirait comme appartenant au précédent.
///
/// La remontée s'arrête à toute ligne qui n'est pas une séquence d'après-match :
/// un recrutement ou un achat de staff n'appartient à aucun match, et se
/// laisserait absorber par le suivant.
///
/// Ce qu'elle ne sait pas faire : deux matchs qui se suivent sans achat de coups
/// de pouce sur le premier voient la recette du premier remonter dans la période
/// du second. Rien dans les données ne les sépare.
///
/// **Rien d'autre ne la borne, et rien d'autre n'est nécessaire** : la période
/// précédente s'arrête sur sa propre ligne à contexte, et l'ouverture sur la
/// dotation — dont le service garantit la présence en tête, faute de quoi il
/// refuse le relevé entier (`MissingOpeningEntry`). Un plancher d'indice en plus
/// n'aurait été atteint par aucun cas ; il ne s'agirait pas de prudence mais de
/// code que rien ne peut vérifier.
fn remonter_la_sequence(lines: &[StatementLine], depuis: usize) -> usize {
    let mut debut = depuis;
    while debut > 0 && appartient_a_l_apres_match(&lines[debut - 1]) {
        debut -= 1;
    }
    debut
}

fn appartient_a_l_apres_match(line: &StatementLine) -> bool {
    line.match_context.is_none()
        && matches!(
            line.reason,
            MovementReason::MatchIncome | MovementReason::CostlyMistake
        )
}

fn ouverture() -> GroupVm {
    GroupVm {
        heading: None,
        rows: Vec::new(),
    }
}

fn periode(titre: &str) -> GroupVm {
    GroupVm {
        heading: Some(titre.to_string()),
        rows: Vec::new(),
    }
}

fn titre_de_periode(ctx: &LineMatchContext) -> String {
    format!("{} — contre {}", ctx.round_name, ctx.opponent_name)
}

// ── Une ligne ─────────────────────────────────────────────────────────────────

fn ligne(l: &StatementLine) -> MovementRowVm {
    let label = libelle(l.reason);
    MovementRowVm {
        date_label: format_date(l.occurred_at),
        icon: emoji(l.reason),
        label: label.to_string(),
        detail: detail(&l.detail, label),
        amount_label: montant(l.direction, l.amount),
        balance_label: format!("{} kPo", l.balance_after),
        kind: nature(l.reason, l.direction),
        is_credit: l.direction == MovementDirection::Credit,
    }
}

/// **Un détail qui répète le libellé n'apprend rien.** Les coups de pouce sont
/// aujourd'hui dans ce cas — l'événement ne porte pas la liste de ce qui a été
/// acheté, seulement le fait. Le repli est écrit une fois, pour tous les motifs :
/// c'est la prochaine collision qu'il attrape, pas seulement celle-ci.
fn detail(detail: &str, label: &str) -> Option<String> {
    match detail.is_empty() || detail == label {
        true => None,
        false => Some(detail.to_string()),
    }
}

/// Le signe appartient au montant. Le **moins typographique** (U+2212) et non le
/// trait d'union : c'est celui de la maquette, et le seul qui s'aligne avec les
/// chiffres.
fn montant(direction: MovementDirection, amount: i32) -> String {
    match direction {
        MovementDirection::Credit => format!("+{amount} kPo"),
        MovementDirection::Debit => format!("−{amount} kPo"),
    }
}

/// Ce que la ligne est dans le relevé — ouvre, entre, sort, ou défait.
///
/// Une bourde coûteuse est un débit : elle sort de l'argent. Son emoji dit déjà
/// la mésaventure, et lui donner une nature à part ferait dire à l'énumération
/// le motif, qu'elle ne doit pas dire.
fn nature(reason: MovementReason, direction: MovementDirection) -> RowKind {
    match reason {
        MovementReason::InitialEndowment => RowKind::Opening,
        MovementReason::MatchIncomeReverted | MovementReason::InducementRefunded => {
            RowKind::Correction
        }
        _ => match direction {
            MovementDirection::Credit => RowKind::Credit,
            MovementDirection::Debit => RowKind::Debit,
        },
    }
}

/// Les huit libellés. Un `match`, pour que le compilateur réclame le neuvième.
fn libelle(reason: MovementReason) -> &'static str {
    match reason {
        MovementReason::InitialEndowment => "Dotation de départ",
        MovementReason::MatchIncome => "Recette d'après-match",
        MovementReason::MatchIncomeReverted => "Recette annulée",
        MovementReason::CostlyMistake => "Bourde coûteuse",
        MovementReason::InducementPurchase => "Coups de pouce",
        MovementReason::InducementRefunded => "Coups de pouce remboursés",
        MovementReason::PlayerRecruitment => "Recrutement",
        MovementReason::StaffPurchase => "Personnel",
    }
}

fn emoji(reason: MovementReason) -> &'static str {
    match reason {
        MovementReason::InitialEndowment => "🏁",
        MovementReason::MatchIncome => "💰",
        MovementReason::MatchIncomeReverted => "↩️",
        MovementReason::CostlyMistake => "💸",
        MovementReason::InducementPurchase => "🎯",
        MovementReason::InducementRefunded => "↩️",
        MovementReason::PlayerRecruitment => "🧍",
        MovementReason::StaffPurchase => "🧢",
    }
}

/// « 12 août » — jour et mois. L'heure n'apprend rien sur un mouvement de
/// caisse, et l'année encombrerait : un relevé de saison ne couvre pas deux ans.
///
/// Recopié de `ranking/io/web/manual_points/builders.rs` : même besoin, autre
/// BC. La souveraineté des BCs interdit de le partager, et un module commun
/// pour douze noms de mois créerait l'adhérence qu'elle proscrit.
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
    use crate::app::teams::use_cases::treasury_statement_service::LineMatchContext;
    use time::macros::datetime;

    fn ligne_brute(
        reason: MovementReason,
        direction: MovementDirection,
        amount: i32,
        balance_after: i32,
        detail: &str,
    ) -> StatementLine {
        StatementLine {
            direction,
            amount,
            reason,
            balance_after,
            occurred_at: datetime!(2026-08-12 10:00 UTC),
            detail: detail.to_string(),
            match_context: None,
        }
    }

    fn dotation(montant: i32) -> StatementLine {
        ligne_brute(
            MovementReason::InitialEndowment,
            MovementDirection::Credit,
            montant,
            montant,
            "Création de l'équipe",
        )
    }

    fn contexte(journee: &str, adversaire: &str) -> LineMatchContext {
        LineMatchContext {
            round_name: journee.to_string(),
            opponent_name: adversaire.to_string(),
            score: None,
        }
    }

    fn releve(lines: Vec<StatementLine>) -> TreasuryStatement {
        let opening = lines.first().map(|l| l.balance_after).unwrap_or(0);
        let earned = lines
            .iter()
            .filter(|l| l.direction == MovementDirection::Credit)
            .map(|l| l.amount)
            .sum();
        let spent = lines
            .iter()
            .filter(|l| l.direction == MovementDirection::Debit)
            .map(|l| l.amount)
            .sum();
        TreasuryStatement {
            opening,
            balance: lines.last().map(|l| l.balance_after).unwrap_or(0),
            earned,
            spent,
            lines,
        }
    }

    // ── Le bandeau ────────────────────────────────────────────────────────────

    /// **L'égalité que le bandeau affiche doit être vraie.**
    ///
    /// `dotation + encaissé − dépensé = solde` est écrite en toutes lettres à
    /// l'écran, avec ses quatre termes côte à côte : une erreur de composition
    /// s'y lit à l'œil nu. Le piège est que `earned` du service **compte la
    /// dotation** — l'oublier ferait afficher une équation fausse.
    #[test]
    fn l_equation_du_bandeau_est_vraie() {
        let vm = build_treasury_vm(&releve(vec![
            dotation(510),
            ligne_brute(
                MovementReason::MatchIncome,
                MovementDirection::Credit,
                60,
                570,
                "Victoire",
            ),
            ligne_brute(
                MovementReason::PlayerRecruitment,
                MovementDirection::Debit,
                90,
                480,
                "Gwenn, Passeuse — n° 7",
            ),
        ]));
        let s = &vm.summary;

        assert_eq!(
            (s.opening_kpo, s.credited_kpo, s.debited_kpo),
            (510, 60, 90)
        );
        assert_eq!(s.balance_kpo, 480);
        assert_eq!(
            s.opening_kpo + s.credited_kpo - s.debited_kpo,
            s.balance_kpo,
            "dotation + encaissé − dépensé doit valoir le solde"
        );
    }

    /// La dotation a sa propre colonne : la compter aussi dans « encaissé » la
    /// ferait apparaître deux fois dans une équation qui doit tomber juste.
    #[test]
    fn l_encaisse_exclut_la_dotation() {
        let vm = build_treasury_vm(&releve(vec![
            dotation(510),
            ligne_brute(
                MovementReason::MatchIncome,
                MovementDirection::Credit,
                60,
                570,
                "Victoire",
            ),
        ]));

        assert_eq!(vm.summary.credited_kpo, 60);
    }

    // ── L'état vide ───────────────────────────────────────────────────────────

    /// Une équipe qui vient d'être créée : sa dotation, et rien d'autre.
    #[test]
    fn la_dotation_seule_est_un_releve_vide() {
        let vm = build_treasury_vm(&releve(vec![dotation(510)]));

        assert!(vm.is_opening_only);
        assert_eq!(vm.movement_count, 1);
        // Contre-épreuve : le bandeau reste rempli, l'équipe a bien 510 kPo.
        assert_eq!(vm.summary.balance_kpo, 510);
    }

    /// Un mouvement de plus, et le tableau reprend la main.
    #[test]
    fn un_seul_mouvement_de_plus_fait_sortir_de_l_etat_vide() {
        let vm = build_treasury_vm(&releve(vec![
            dotation(510),
            ligne_brute(
                MovementReason::StaffPurchase,
                MovementDirection::Debit,
                50,
                460,
                "Apothicaire × 1",
            ),
        ]));

        assert!(!vm.is_opening_only);
        assert_eq!(vm.movement_count, 2);
    }

    // ── Les périodes ──────────────────────────────────────────────────────────

    /// L'ouverture n'a pas de journée : le gabarit n'y met aucun séparateur.
    #[test]
    fn le_groupe_d_ouverture_n_a_pas_de_titre() {
        let vm = build_treasury_vm(&releve(vec![
            dotation(510),
            ligne_brute(
                MovementReason::PlayerRecruitment,
                MovementDirection::Debit,
                90,
                420,
                "Gwenn, Passeuse — n° 7",
            ),
        ]));

        assert_eq!(vm.groups.len(), 1);
        assert!(vm.groups[0].heading.is_none());
        assert_eq!(vm.groups[0].rows.len(), 2);
    }

    /// **Le titre ouvre une période ; il n'étiquette pas des lignes.**
    ///
    /// C'est la seule lecture que les données permettent : la recette de match
    /// ne porte aucun identifiant de rapport, donc aucun contexte. Elle doit
    /// malgré tout se ranger sous la journée qui vient de commencer.
    #[test]
    fn une_ligne_sans_contexte_rejoint_la_periode_ouverte() {
        let mut coups = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            100,
            410,
            "Coups de pouce",
        );
        coups.match_context = Some(contexte("Journée 1", "Les Trolls du Bief"));

        let vm = build_treasury_vm(&releve(vec![
            dotation(510),
            coups,
            // Sans contexte — c'est le cas de toutes les recettes de match.
            ligne_brute(
                MovementReason::MatchIncome,
                MovementDirection::Credit,
                60,
                470,
                "Victoire",
            ),
        ]));

        assert_eq!(vm.groups.len(), 2);
        assert!(vm.groups[0].heading.is_none());
        assert_eq!(
            vm.groups[1].heading.as_deref(),
            Some("Journée 1 — contre Les Trolls du Bief")
        );
        assert_eq!(
            vm.groups[1].rows.len(),
            2,
            "la recette doit rejoindre la journée ouverte, pas rester seule"
        );
    }

    /// **La recette rejoint le match dont elle vient, pas le précédent.**
    ///
    /// Le grand livre écrit la recette *avant* le paiement des coups de pouce —
    /// 110 équipes sur 110 dans la base du 2026-08-30. Sans la remontée, le
    /// titre « Journée 1 » s'ouvrirait après la recette de la journée 1, qui se
    /// lirait alors comme appartenant à ce qui précède.
    #[test]
    fn la_recette_remonte_dans_la_periode_de_son_match() {
        let mut coups = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            15,
            555,
            "Coups de pouce",
        );
        coups.match_context = Some(contexte("Journée 1", "Les Trolls du Bief"));

        let vm = build_treasury_vm(&releve(vec![
            dotation(565),
            ligne_brute(
                MovementReason::MatchIncome,
                MovementDirection::Credit,
                5,
                570,
                "Match nul",
            ),
            coups,
        ]));

        assert_eq!(vm.groups.len(), 2);
        assert_eq!(
            vm.groups[0].rows.len(),
            1,
            "l'ouverture ne garde que la dotation"
        );
        let labels: Vec<&str> = vm.groups[1].rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["Recette d'après-match", "Coups de pouce"]);
    }

    /// **La remontée s'arrête à ce qui n'appartient à aucun match.** Un
    /// recrutement fait entre deux matchs ne doit pas se faire absorber par le
    /// suivant : il reste dans la période où il a eu lieu.
    #[test]
    fn un_recrutement_ne_se_fait_pas_absorber_par_le_match_suivant() {
        let mut coups = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            15,
            455,
            "Coups de pouce",
        );
        coups.match_context = Some(contexte("Journée 2", "Les Griffons d'Argent"));

        let vm = build_treasury_vm(&releve(vec![
            dotation(565),
            ligne_brute(
                MovementReason::PlayerRecruitment,
                MovementDirection::Debit,
                90,
                475,
                "Gwenn, Passeuse — n° 7",
            ),
            coups,
        ]));

        assert_eq!(vm.groups[0].rows.len(), 2, "le recrutement reste en amont");
        assert_eq!(vm.groups[1].rows.len(), 1);
    }

    /// La dotation ne remonte jamais : c'est le point de départ du relevé, et
    /// elle n'appartient à aucun match.
    #[test]
    fn la_dotation_ne_remonte_jamais_dans_une_periode() {
        let mut coups = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            15,
            550,
            "Coups de pouce",
        );
        coups.match_context = Some(contexte("Journée 1", "Les Trolls du Bief"));

        let vm = build_treasury_vm(&releve(vec![dotation(565), coups]));

        assert!(vm.groups[0].heading.is_none());
        assert_eq!(vm.groups[0].rows[0].label, "Dotation de départ");
        assert_eq!(vm.groups[1].rows.len(), 1);
    }

    /// **La remontée s'arrête à la période précédente.** Sans cette borne, la
    /// recette du second match traverserait la ligne à contexte du premier et
    /// viderait sa période de ce qui l'ancrait.
    #[test]
    fn la_remontee_ne_traverse_pas_la_periode_precedente() {
        let mut coups1 = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            15,
            555,
            "Coups de pouce",
        );
        coups1.match_context = Some(contexte("Journée 1", "Les Trolls du Bief"));
        let mut coups2 = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            15,
            545,
            "Coups de pouce",
        );
        coups2.match_context = Some(contexte("Journée 2", "Les Griffons d'Argent"));

        let vm = build_treasury_vm(&releve(vec![
            dotation(565),
            ligne_brute(
                MovementReason::MatchIncome,
                MovementDirection::Credit,
                5,
                570,
                "Match nul",
            ),
            coups1,
            ligne_brute(
                MovementReason::MatchIncome,
                MovementDirection::Credit,
                10,
                565,
                "Victoire",
            ),
            coups2,
        ]));

        let tailles: Vec<usize> = vm.groups.iter().map(|g| g.rows.len()).collect();
        assert_eq!(tailles, vec![1, 2, 2], "{:?}", tailles);
        assert!(vm.groups[1]
            .heading
            .as_deref()
            .unwrap()
            .starts_with("Journée 1"));
        assert!(vm.groups[2]
            .heading
            .as_deref()
            .unwrap()
            .starts_with("Journée 2"));
    }

    /// **Une ligne qui connaît déjà son match ne se fait absorber par aucun
    /// autre.**
    ///
    /// Aujourd'hui aucune bourde coûteuse ne porte d'identifiant de rapport —
    /// seuls trois événements en portent — mais rien n'empêchera un jour
    /// `CostlyMistakesApplied` d'en porter un. Le motif seul ne suffirait alors
    /// plus : la bourde de la journée 1 remonterait dans la période de la
    /// journée 2, en emportant son propre titre.
    #[test]
    fn une_ligne_deja_rattachee_ne_remonte_pas_dans_un_autre_match() {
        let mut bourde = ligne_brute(
            MovementReason::CostlyMistake,
            MovementDirection::Debit,
            20,
            545,
            "Catastrophe — jet de 3",
        );
        bourde.match_context = Some(contexte("Journée 1", "Les Trolls du Bief"));
        let mut coups = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            15,
            530,
            "Coups de pouce",
        );
        coups.match_context = Some(contexte("Journée 2", "Les Griffons d'Argent"));

        let vm = build_treasury_vm(&releve(vec![dotation(565), bourde, coups]));

        let titres: Vec<Option<&str>> = vm.groups.iter().map(|g| g.heading.as_deref()).collect();
        assert_eq!(
            titres,
            vec![
                None,
                Some("Journée 1 — contre Les Trolls du Bief"),
                Some("Journée 2 — contre Les Griffons d'Argent"),
            ],
            "la bourde doit garder sa propre journée"
        );
    }

    /// Deux matchs, deux périodes — et la seconde ne reprend pas la première.
    #[test]
    fn un_match_different_ouvre_une_nouvelle_periode() {
        let mut j1 = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            100,
            410,
            "Coups de pouce",
        );
        j1.match_context = Some(contexte("Journée 1", "Les Trolls du Bief"));
        let mut j2 = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            15,
            395,
            "Coups de pouce",
        );
        j2.match_context = Some(contexte("Journée 2", "Les Griffons d'Argent"));

        let vm = build_treasury_vm(&releve(vec![dotation(510), j1, j2]));

        let titres: Vec<Option<&str>> = vm.groups.iter().map(|g| g.heading.as_deref()).collect();
        assert_eq!(
            titres,
            vec![
                None,
                Some("Journée 1 — contre Les Trolls du Bief"),
                Some("Journée 2 — contre Les Griffons d'Argent"),
            ]
        );
    }

    /// Deux lignes du **même** match ne doivent pas ouvrir deux périodes : la
    /// paie des coups de pouce et leur remboursement encadrent le même match.
    #[test]
    fn deux_lignes_du_meme_match_restent_dans_une_periode() {
        let mut paie = ligne_brute(
            MovementReason::InducementPurchase,
            MovementDirection::Debit,
            15,
            495,
            "Coups de pouce",
        );
        paie.match_context = Some(contexte("Journée 2", "Les Griffons d'Argent"));
        let mut rendu = ligne_brute(
            MovementReason::InducementRefunded,
            MovementDirection::Credit,
            15,
            510,
            "Rendus avec l'annulation du rapport",
        );
        rendu.match_context = Some(contexte("Journée 2", "Les Griffons d'Argent"));

        let vm = build_treasury_vm(&releve(vec![dotation(510), paie, rendu]));

        assert_eq!(vm.groups.len(), 2);
        assert_eq!(vm.groups[1].rows.len(), 2);
    }

    // ── Une ligne ─────────────────────────────────────────────────────────────

    /// Le signe appartient au montant, jamais au solde — et c'est le moins
    /// typographique, celui qui s'aligne avec les chiffres.
    #[test]
    fn le_montant_porte_son_signe_et_le_solde_non() {
        let vm = build_treasury_vm(&releve(vec![
            dotation(510),
            ligne_brute(
                MovementReason::PlayerRecruitment,
                MovementDirection::Debit,
                90,
                420,
                "Gwenn, Passeuse — n° 7",
            ),
        ]));
        let rows = &vm.groups[0].rows;

        assert_eq!(rows[0].amount_label, "+510 kPo");
        assert_eq!(rows[0].balance_label, "510 kPo");
        assert_eq!(rows[1].amount_label, "−90 kPo");
        assert_eq!(rows[1].balance_label, "420 kPo");
        assert!(
            rows[1].amount_label.starts_with('\u{2212}'),
            "le moins doit être U+2212, pas un trait d'union : {}",
            rows[1].amount_label
        );
    }

    /// **Un détail qui répète le libellé n'apprend rien.** C'est le cas des
    /// coups de pouce : l'événement ne porte pas ce qui a été acheté, seulement
    /// le fait, et le service rend donc « Coups de pouce » deux fois.
    #[test]
    fn un_detail_qui_repete_le_libelle_disparait() {
        let vm = build_treasury_vm(&releve(vec![
            dotation(510),
            ligne_brute(
                MovementReason::InducementPurchase,
                MovementDirection::Debit,
                100,
                410,
                "Coups de pouce",
            ),
        ]));
        let coups = &vm.groups[0].rows[1];

        assert_eq!(coups.label, "Coups de pouce");
        assert_eq!(coups.detail, None);
        // Contre-épreuve : un détail qui apprend quelque chose est conservé.
        assert_eq!(
            vm.groups[0].rows[0].detail.as_deref(),
            Some("Création de l'équipe")
        );
    }

    #[test]
    fn un_detail_vide_disparait_aussi() {
        let vm = build_treasury_vm(&releve(vec![
            dotation(510),
            ligne_brute(
                MovementReason::CostlyMistake,
                MovementDirection::Debit,
                205,
                305,
                "",
            ),
        ]));

        assert_eq!(vm.groups[0].rows[1].detail, None);
    }

    /// La date se lit « 12 août » — ni heure, ni année.
    #[test]
    fn la_date_est_le_jour_et_le_mois() {
        let vm = build_treasury_vm(&releve(vec![dotation(510)]));

        assert_eq!(vm.groups[0].rows[0].date_label, "12 août");
    }

    // ── Les huit motifs ───────────────────────────────────────────────────────

    /// Les huit libellés et les huit emojis, en un seul endroit vérifiable.
    ///
    /// Ils passent par `build_treasury_vm` et non par `libelle`/`emoji`
    /// directement : c'est le chemin réel, et il prouve du même coup que la
    /// nature de la ligne suit le motif.
    #[test]
    fn les_huit_motifs_ont_leur_libelle_leur_emoji_et_leur_nature() {
        let attendu = [
            (
                MovementReason::InitialEndowment,
                MovementDirection::Credit,
                "Dotation de départ",
                "🏁",
                RowKind::Opening,
            ),
            (
                MovementReason::MatchIncome,
                MovementDirection::Credit,
                "Recette d'après-match",
                "💰",
                RowKind::Credit,
            ),
            (
                MovementReason::MatchIncomeReverted,
                MovementDirection::Debit,
                "Recette annulée",
                "↩️",
                RowKind::Correction,
            ),
            (
                MovementReason::CostlyMistake,
                MovementDirection::Debit,
                "Bourde coûteuse",
                "💸",
                RowKind::Debit,
            ),
            (
                MovementReason::InducementPurchase,
                MovementDirection::Debit,
                "Coups de pouce",
                "🎯",
                RowKind::Debit,
            ),
            (
                MovementReason::InducementRefunded,
                MovementDirection::Credit,
                "Coups de pouce remboursés",
                "↩️",
                RowKind::Correction,
            ),
            (
                MovementReason::PlayerRecruitment,
                MovementDirection::Debit,
                "Recrutement",
                "🧍",
                RowKind::Debit,
            ),
            (
                MovementReason::StaffPurchase,
                MovementDirection::Debit,
                "Personnel",
                "🧢",
                RowKind::Debit,
            ),
        ];

        for (motif, direction, libelle_attendu, emoji_attendu, nature_attendue) in attendu {
            let vm = build_treasury_vm(&releve(vec![
                dotation(510),
                ligne_brute(motif, direction, 10, 500, "détail quelconque"),
            ]));
            let row = &vm.groups[0].rows[1];

            assert_eq!(row.label, libelle_attendu, "libellé de {motif:?}");
            assert_eq!(row.icon, emoji_attendu, "emoji de {motif:?}");
            assert_eq!(row.kind, nature_attendue, "nature de {motif:?}");
            assert_eq!(
                row.is_credit,
                direction == MovementDirection::Credit,
                "sens de {motif:?}"
            );
        }
    }

    /// Le garde d'exhaustivité : **le compilateur** réclame la mise à jour du
    /// tableau ci-dessus quand un motif apparaît.
    ///
    /// Sans lui, le test précédent énumérerait huit motifs à la main et
    /// passerait en ignorant le neuvième — un libellé manquant ne se verrait
    /// alors qu'à l'écran. C'est exactement le trou qu'a montré la carte 435.
    #[allow(dead_code)]
    fn garde_d_exhaustivite(motif: MovementReason) {
        match motif {
            MovementReason::InitialEndowment => (),
            MovementReason::MatchIncome => (),
            MovementReason::MatchIncomeReverted => (),
            MovementReason::CostlyMistake => (),
            MovementReason::InducementPurchase => (),
            MovementReason::InducementRefunded => (),
            MovementReason::PlayerRecruitment => (),
            MovementReason::StaffPurchase => (),
        }
    }
}
