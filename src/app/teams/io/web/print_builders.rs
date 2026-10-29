//! La construction de l'en-tête d'une fiche imprimée (carte 597).
//!
//! Un builder plutôt qu'un `from_domain()` : la vue lit le catalogue du roster,
//! que porte un port, et le `CLAUDE.md` réserve le constructeur co-localisé aux
//! vues qui se construisent depuis le seul domaine.
//!
//! **Les prix du staff viennent du catalogue**, comme dans le calcul de la VE.
//! La fiche d'équipe les écrit encore en dur (`StaffVm` : 50, 10, 10) ; l'écart
//! est noté hors périmètre dans la spec de l'impression.

use crate::app::teams::io::web::widgets::print_preview_widget::SheetRole;
use crate::app::teams::io::web::widgets::print_sheet_head_widget::{
    StaffPrintLineVm, TeamSheetHeadVm,
};
use crate::app::teams::ports::RosterCatalogDto;
use crate::app::teams::use_cases::team_print_service::SheetHeadData;

pub fn build_sheet_head_vm(data: &SheetHeadData, role: SheetRole) -> TeamSheetHeadVm {
    let team = &data.team;
    let catalog = data.catalog.as_ref();
    TeamSheetHeadVm {
        role_label: role_label(role),
        name: team.name.to_string(),
        initials: team.initials.clone(),
        logo_url: team.logo_url.as_deref().map(logo),
        roster_name: team.roster_name.to_string(),
        coach_name: team.coach_name.clone(),
        competition_name: team.competition_name.clone(),
        season_name: team.season_name.clone(),
        team_value_kpo: team.team_value.0,
        treasury_kpo: team.treasury().0,
        dedicated_fans: team.dedicated_fans.into_inner(),
        alignable: data.squad.iter().filter(|m| m.presence.alignable()).count(),
        squad_size: data.squad.len(),
        rerolls: team.rerolls.0,
        reroll_price_kpo: catalog.map(|c| c.reroll_base_cost),
        staff: staff_lines(data, catalog),
    }
}

fn role_label(role: SheetRole) -> &'static str {
    match role {
        SheetRole::Own => "Mon équipe",
        SheetRole::Other => "Adversaire",
    }
}

fn logo(url: &str) -> String {
    crate::app::shared_kernel::identity::cloudinary::transform(
        url,
        "c_fill,w_120,h_120,q_auto,f_auto",
    )
}

fn staff_lines(data: &SheetHeadData, catalog: Option<&RosterCatalogDto>) -> Vec<StaffPrintLineVm> {
    // Sans catalogue, aucun prix : un « 0 kPo » serait plausible, donc faux.
    let price = |uid: &str| catalog.map(|c| c.staff_price(uid));
    let team = &data.team;
    vec![
        StaffPrintLineVm {
            label: "Apothicaire",
            quantity: team.apothecaries.0,
            unit_price_kpo: price("APOTHECARY"),
        },
        StaffPrintLineVm {
            label: "Assistants",
            quantity: team.assistants.0,
            unit_price_kpo: price("COACH_ASSISTANTS"),
        },
        StaffPrintLineVm {
            label: "Pom-pom girls",
            quantity: team.cheerleaders.0,
            unit_price_kpo: price("CHEERLEADERS"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::teams::domain::basket::SquadPresence;
    use crate::app::teams::io::web::tests::fixtures::equipe_de_test;
    use crate::app::teams::ports::{SquadMemberDto, StaffPriceDto};

    fn membre(presence: SquadPresence) -> SquadMemberDto {
        SquadMemberDto {
            player_id: "p".into(),
            roster_line_id: "l".into(),
            jersey: None,
            personal_name: String::new(),
            position_name: String::new(),
            spp: 0,
            value_kpo: 50,
            presence,
            is_temporary: false,
        }
    }

    fn catalogue() -> RosterCatalogDto {
        let prix = |uid: &str, price| StaffPriceDto {
            uid: uid.into(),
            name: uid.into(),
            price,
            max_quantity: 1,
        };
        RosterCatalogDto {
            logo: None,
            linemen_are_free: false,
            reroll_base_cost: 60,
            positions: vec![],
            cross_limits: vec![],
            allowed_staff: vec![],
            staff_prices: vec![
                prix("APOTHECARY", 50),
                prix("COACH_ASSISTANTS", 15),
                prix("CHEERLEADERS", 12),
            ],
        }
    }

    fn donnees(squad: Vec<SquadMemberDto>) -> SheetHeadData {
        SheetHeadData {
            team: equipe_de_test(),
            catalog: Some(catalogue()),
            squad,
        }
    }

    /// R8 : la trésorerie s'imprime sur toutes les fiches.
    #[test]
    fn la_tresorerie_s_imprime_sur_toutes_les_fiches() {
        let d = donnees(vec![]);
        assert_eq!(build_sheet_head_vm(&d, SheetRole::Own).treasury_kpo, 1000);
        assert_eq!(build_sheet_head_vm(&d, SheetRole::Other).treasury_kpo, 1000);
    }

    /// Les prix du staff sont ceux du catalogue, pas les 50 / 10 / 10 de la fiche.
    #[test]
    fn les_prix_du_staff_sont_lus_dans_le_catalogue() {
        let vm = build_sheet_head_vm(&donnees(vec![]), SheetRole::Own);
        let prix: Vec<Option<u32>> = vm.staff.iter().map(|l| l.unit_price_kpo).collect();
        assert_eq!(prix, vec![Some(50), Some(15), Some(12)]);
        assert_eq!(vm.reroll_price_kpo, Some(60));
    }

    /// R14 : les alignables sont ceux que le domaine déclare tels.
    #[test]
    fn les_alignables_suivent_la_presence_du_domaine() {
        let squad = vec![
            membre(SquadPresence::Alignable),
            membre(SquadPresence::Alignable),
            membre(SquadPresence::Empeche),
        ];
        let vm = build_sheet_head_vm(&donnees(squad), SheetRole::Own);
        assert_eq!((vm.alignable, vm.squad_size), (2, 3));
    }

    #[test]
    fn sans_catalogue_aucun_prix_n_est_invente() {
        let mut d = donnees(vec![]);
        d.catalog = None;
        let vm = build_sheet_head_vm(&d, SheetRole::Own);
        assert_eq!(vm.reroll_price_kpo, None);
        assert!(vm.staff.iter().all(|l| l.unit_price_kpo.is_none()));
    }
}
