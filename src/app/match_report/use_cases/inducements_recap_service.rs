//! Le récapitulatif des coups de pouce des deux camps (carte 628, R6).
//!
//! Service d'hydratation : le domaine dit ce qui a été acheté, sa nature, son
//! total, le budget et qui l'a validé ; ce service n'y ajoute que les noms,
//! lus par les ports — libellés et vedettes au catalogue de `references`,
//! postes des mercenaires au roster de l'équipe, coachs par `ICoachDataPort`.

use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::{InducementPurchase, PurchaseKind};
use crate::app::match_report::ports::{ICoachDataPort, IReferenceCatalogPort, ITeamDataPort};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;
use std::collections::{HashMap, HashSet};

/// Les ports que le récapitulatif interroge.
pub struct RecapPorts<'a> {
    pub team_data: &'a dyn ITeamDataPort,
    pub coach_data: &'a dyn ICoachDataPort,
    pub catalog: &'a dyn IReferenceCatalogPort,
}

/// Les deux camps, le favori d'abord.
pub struct InducementsRecap {
    pub sides: Vec<RecapSide>,
}

pub struct RecapSide {
    pub team_id: String,
    pub team_name: String,
    pub is_topdog: bool,
    pub lines: Vec<RecapLine>,
    pub spent: u32,
    pub budget: u32,
    pub recorded_by: Option<RecordedBy>,
}

pub struct RecapLine {
    pub name: String,
    pub qty: u8,
    pub kind: PurchaseKind,
    pub total_cost: u32,
}

/// L'auteur d'une validation, vu de celui qui lit la page.
#[derive(Debug, PartialEq, Eq)]
pub enum RecordedBy {
    Viewer,
    Coach(String),
}

// arch:no-instrument — service d'hydratation : assemble le récapitulatif, sans intention métier
pub async fn build(
    pm: &MatchReportPreMatch,
    viewer: CoachId,
    ports: &RecapPorts<'_>,
) -> InducementsRecap {
    let mut sides = Vec::new();
    for team_id in [pm.topdog_team_id(), pm.underdog_team_id()] {
        sides.push(build_side(pm, team_id, viewer, ports).await);
    }
    InducementsRecap { sides }
}

async fn build_side(
    pm: &MatchReportPreMatch,
    team_id: &TeamId,
    viewer: CoachId,
    ports: &RecapPorts<'_>,
) -> RecapSide {
    let id = team_id.to_string();
    let info = ports
        .team_data
        .find_team_info(&id)
        .await
        .unwrap_or_default();
    let treasury = ports.team_data.find_team_treasury(&id).await.unwrap_or(0);
    let author = pm.inducements_recorded_by(team_id);
    RecapSide {
        lines: recap_lines(pm, team_id, &info.roster_id, ports).await,
        team_id: id,
        team_name: info.team_name,
        is_topdog: team_id == pm.topdog_team_id(),
        spent: pm.inducements_total_for(team_id),
        budget: pm.inducement_budget_for(team_id, treasury),
        recorded_by: recorded_by(author, viewer, ports.coach_data).await,
    }
}

async fn recap_lines(
    pm: &MatchReportPreMatch,
    team_id: &TeamId,
    roster_id: &str,
    ports: &RecapPorts<'_>,
) -> Vec<RecapLine> {
    let purchases = pm.purchases_for(team_id);
    let names = Names::load(purchases, team_id, roster_id, ports).await;
    purchases
        .iter()
        .map(|p| {
            let kind = pm.purchase_kind(team_id, p);
            RecapLine {
                name: names.of(p, &kind),
                qty: p.qty.into_inner(),
                total_cost: p.total_cost(),
                kind,
            }
        })
        .collect()
}

/// Une ligne d'une sélection validée, nommée : ce que l'annonce de la
/// validation porte hors du BC (carte 643).
#[derive(Debug, PartialEq, Eq)]
pub struct LabelledPurchase {
    pub label: String,
    pub qty: u8,
    pub kind: PurchaseKind,
}

/// Nomme les achats d'une sélection **sans l'agrégat** : la nature d'une
/// vedette se lit au catalogue (`find_star_players`), et non sur les
/// engagements de l'avant-match, que le publisher ne relit pas.
// arch:no-instrument — service d'hydratation : nomme une sélection, sans intention métier
pub async fn labelled_purchases(
    purchases: &[InducementPurchase],
    team_id: &TeamId,
    roster_id: &str,
    ports: &RecapPorts<'_>,
) -> Vec<LabelledPurchase> {
    let names = Names::load(purchases, team_id, roster_id, ports).await;
    purchases
        .iter()
        .map(|p| {
            let kind = names.kind_of(p);
            LabelledPurchase {
                label: names.of(p, &kind),
                qty: p.qty.into_inner(),
                kind,
            }
        })
        .collect()
}

async fn recorded_by(
    author: Option<CoachId>,
    viewer: CoachId,
    coach_data: &dyn ICoachDataPort,
) -> Option<RecordedBy> {
    let author = author?;
    if author == viewer {
        return Some(RecordedBy::Viewer);
    }
    coach_data
        .find_coach_name(&author.to_string())
        .await
        .map(RecordedBy::Coach)
}

/// Les noms d'une liste d'achats : coups de pouce et vedettes par leur
/// identifiant, mercenaires par leur poste. Un nom introuvable se rend par
/// l'identifiant — visible, plutôt qu'une ligne qui disparaîtrait.
struct Names {
    by_uid: HashMap<String, String>,
    by_position: HashMap<String, String>,
    star_uids: HashSet<String>,
}

impl Names {
    async fn load(
        purchases: &[InducementPurchase],
        team_id: &TeamId,
        roster_id: &str,
        ports: &RecapPorts<'_>,
    ) -> Self {
        let uids: Vec<String> = purchases.iter().map(|p| p.uid.0.clone()).collect();
        let labels = ports.catalog.find_inducement_labels(&uids);
        let stars = ports.catalog.find_star_players(&uids, roster_id);
        let star_uids = stars.iter().map(|s| s.uid.clone()).collect();
        let by_uid = labels
            .into_iter()
            .map(|l| (l.uid, l.name))
            .chain(stars.into_iter().map(|s| (s.uid, s.name)))
            .collect();
        Self {
            by_uid,
            by_position: positions(purchases, team_id, ports.team_data).await,
            star_uids,
        }
    }

    /// La nature d'un achat d'après son identifiant et le catalogue.
    fn kind_of(&self, purchase: &InducementPurchase) -> PurchaseKind {
        if let Some(kind) = purchase.mercenary_kind() {
            return kind;
        }
        match self.star_uids.contains(&purchase.uid.0) {
            true => PurchaseKind::StarPlayer,
            false => PurchaseKind::Inducement,
        }
    }

    fn of(&self, purchase: &InducementPurchase, kind: &PurchaseKind) -> String {
        let found = match kind {
            PurchaseKind::Mercenary { position_uid, .. } => self.by_position.get(position_uid),
            PurchaseKind::Inducement | PurchaseKind::StarPlayer => self.by_uid.get(&purchase.uid.0),
        };
        found.cloned().unwrap_or_else(|| purchase.uid.0.clone())
    }
}

/// Les postes du roster, lus seulement s'il y a un mercenaire à nommer.
async fn positions(
    purchases: &[InducementPurchase],
    team_id: &TeamId,
    team_data: &dyn ITeamDataPort,
) -> HashMap<String, String> {
    if purchases.iter().all(|p| p.mercenary_kind().is_none()) {
        return HashMap::new();
    }
    team_data
        .find_roster_positions(&team_id.to_string())
        .await
        .into_iter()
        .map(|p| (p.position_uid, p.position_name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{
        InducementCost, InducementQty, TeamValue,
    };
    use crate::app::match_report::ports::{
        InducementLabelDto, JourneymanPositionDto, MatchRemindersDto, RosterPositionDto,
        SppScaleDto, StarPlayerDto, TeamInfoDto,
    };
    use crate::app::match_report::use_cases::test_doubles::pre_match;
    use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
    use async_trait::async_trait;

    struct Teams {
        home: TeamId,
    }

    #[async_trait]
    impl ITeamDataPort for Teams {
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn is_coach_of_team(&self, _: &str, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn find_team_coach(&self, _: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        async fn find_team_info(&self, team_id: &str) -> Option<TeamInfoDto> {
            let home = team_id == self.home.to_string();
            Some(TeamInfoDto {
                team_name: if home { "Nains de Fer" } else { "London Rats" }.into(),
                roster_id: "ROSTER".into(),
                ..Default::default()
            })
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            Some(510)
        }
        async fn find_journeyman_position(&self, _: &str) -> Option<JourneymanPositionDto> {
            None
        }
        async fn find_roster_positions(&self, _: &str) -> Vec<RosterPositionDto> {
            vec![RosterPositionDto {
                position_uid: "dwarf-blocker".into(),
                position_name: "Bloqueur nain".into(),
                base_cost: 70,
                max_qty: 4,
                is_journeyman: false,
                keywords: vec![],
            }]
        }
        async fn find_team_season(&self, _: &str) -> Option<String> {
            None
        }
    }

    struct Catalog;

    impl IReferenceCatalogPort for Catalog {
        fn find_inducement_labels(&self, uids: &[String]) -> Vec<InducementLabelDto> {
            uids.iter()
                .filter(|u| u.as_str() == "WAGON")
                .map(|u| InducementLabelDto {
                    uid: u.clone(),
                    name: "Chariot de Vivres".into(),
                    description: String::new(),
                })
                .collect()
        }
        fn find_star_players(&self, uids: &[String], _: &str) -> Vec<StarPlayerDto> {
            uids.iter()
                .filter(|u| u.as_str() == "MORG")
                .map(|u| StarPlayerDto {
                    uid: u.clone(),
                    name: "Morg 'n' Thorg".into(),
                    player_type: String::new(),
                    cost: 340,
                    ma: 6,
                    st: 6,
                    ag: String::new(),
                    pa: String::new(),
                    av: String::new(),
                    skills: vec![],
                    ability_name: String::new(),
                    ability_description: String::new(),
                })
                .collect()
        }
        fn find_spp_scale(&self, _: &str) -> SppScaleDto {
            SppScaleDto::default()
        }
        fn find_match_reminders(&self) -> MatchRemindersDto {
            MatchRemindersDto::default()
        }
    }

    struct Coaches;

    #[async_trait]
    impl ICoachDataPort for Coaches {
        async fn find_coach_name(&self, _: &str) -> Option<String> {
            Some("Morgane".into())
        }
    }

    fn purchase(uid: &str, qty: u8, cost: u32) -> InducementPurchase {
        InducementPurchase {
            uid: InducementId(uid.into()),
            qty: InducementQty::try_new(qty).unwrap(),
            unit_cost: InducementCost::try_new(cost).unwrap(),
        }
    }

    /// Le domicile favori (valeur plus haute) achète trois choses ; l'outsider
    /// passe.
    fn recorded(viewer: CoachId, other: CoachId) -> MatchReportPreMatch {
        let mut pm = pre_match();
        pm.home_team_value = Some(TeamValue::try_new(1100).unwrap());
        pm.home_inducements = Some(vec![
            purchase("WAGON", 2, 15),
            purchase("MORG", 1, 300),
            purchase("MERCO:dwarf-blocker:lvl1", 1, 150),
        ]);
        pm.star_engagements = vec![(pm.home_team_id.clone(), InducementId("MORG".into()))];
        pm.away_inducements = Some(vec![]);
        pm.home_inducements_recorded_by = Some(viewer);
        pm.away_inducements_recorded_by = Some(other);
        pm
    }

    async fn recap_of(pm: &MatchReportPreMatch, viewer: CoachId) -> InducementsRecap {
        let teams = Teams {
            home: pm.home_team_id.clone(),
        };
        let ports = RecapPorts {
            team_data: &teams,
            coach_data: &Coaches,
            catalog: &Catalog,
        };
        build(pm, viewer, &ports).await
    }

    #[tokio::test]
    async fn le_favori_d_abord_puis_l_outsider() {
        let (viewer, other) = (CoachId::new(), CoachId::new());
        let recap = recap_of(&recorded(viewer, other), viewer).await;
        let names: Vec<_> = recap.sides.iter().map(|s| s.team_name.as_str()).collect();
        assert_eq!(names, ["Nains de Fer", "London Rats"]);
        assert!(recap.sides[0].is_topdog);
        assert!(!recap.sides[1].is_topdog);
    }

    /// Chaque nature a son nom : libellé, vedette, poste du mercenaire.
    #[tokio::test]
    async fn chaque_achat_est_nomme_selon_sa_nature() {
        let (viewer, other) = (CoachId::new(), CoachId::new());
        let recap = recap_of(&recorded(viewer, other), viewer).await;
        let lines = &recap.sides[0].lines;
        let named: Vec<_> = lines.iter().map(|l| (l.name.as_str(), l.qty)).collect();
        assert_eq!(
            named,
            [
                ("Chariot de Vivres", 2),
                ("Morg 'n' Thorg", 1),
                ("Bloqueur nain", 1)
            ]
        );
        assert_eq!(lines[0].kind, PurchaseKind::Inducement);
        assert_eq!(lines[1].kind, PurchaseKind::StarPlayer);
        assert!(matches!(
            lines[2].kind,
            PurchaseKind::Mercenary { skilled: true, .. }
        ));
        assert_eq!(lines[0].total_cost, 30);
    }

    /// Le total et le budget viennent du domaine ; « Passer » donne une liste
    /// vide.
    #[tokio::test]
    async fn le_total_sur_le_budget_et_passer_vide() {
        let (viewer, other) = (CoachId::new(), CoachId::new());
        let recap = recap_of(&recorded(viewer, other), viewer).await;
        assert_eq!(recap.sides[0].spent, 480);
        assert_eq!(recap.sides[0].budget, 510);
        assert!(recap.sides[1].lines.is_empty());
        assert_eq!(recap.sides[1].spent, 0);
    }

    /// Décision 5 : « par vous » pour le lecteur, le nom du coach sinon.
    #[tokio::test]
    async fn l_auteur_de_chaque_validation() {
        let (viewer, other) = (CoachId::new(), CoachId::new());
        let recap = recap_of(&recorded(viewer, other), viewer).await;
        assert_eq!(recap.sides[0].recorded_by, Some(RecordedBy::Viewer));
        assert_eq!(
            recap.sides[1].recorded_by,
            Some(RecordedBy::Coach("Morgane".into()))
        );
    }

    /// Un identifiant inconnu du catalogue reste visible.
    #[tokio::test]
    async fn un_nom_introuvable_se_rend_par_l_identifiant() {
        let viewer = CoachId::new();
        let mut pm = recorded(viewer, viewer);
        pm.away_inducements = Some(vec![purchase("UNKNOWN", 1, 10)]);
        let recap = recap_of(&pm, viewer).await;
        assert_eq!(recap.sides[1].lines[0].name, "UNKNOWN");
    }

    /// Carte 643 : sans l'agrégat, la sélection est nommée et typée comme au
    /// récapitulatif — la vedette reconnue au catalogue.
    #[tokio::test]
    async fn labelled_purchases_name_and_type_each_line_without_the_aggregate() {
        let pm = pre_match();
        let teams = Teams {
            home: pm.home_team_id.clone(),
        };
        let ports = RecapPorts {
            team_data: &teams,
            coach_data: &Coaches,
            catalog: &Catalog,
        };
        let purchases = vec![
            purchase("WAGON", 2, 15),
            purchase("MORG", 1, 300),
            purchase("MERCO:dwarf-blocker:base", 1, 150),
        ];

        let lines = labelled_purchases(&purchases, &pm.home_team_id, "ROSTER", &ports).await;

        let labels: Vec<_> = lines.iter().map(|l| (l.label.as_str(), l.qty)).collect();
        assert_eq!(
            labels,
            [
                ("Chariot de Vivres", 2),
                ("Morg 'n' Thorg", 1),
                ("Bloqueur nain", 1)
            ]
        );
        assert_eq!(lines[0].kind, PurchaseKind::Inducement);
        assert_eq!(lines[1].kind, PurchaseKind::StarPlayer);
        assert!(matches!(
            lines[2].kind,
            PurchaseKind::Mercenary { skilled: false, .. }
        ));
    }
}
