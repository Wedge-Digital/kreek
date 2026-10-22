//! Les coups de pouce et star players disponibles d'une impression (carte 607).
//!
//! **Le fragment rend ses propres feuilles** — le contrat des fragments
//! d'impression multi-pages de la page « Imprimer » de `teams`. Coups de pouce
//! et star players partagent leurs feuilles quand les deux options sont
//! cochées.
//!
//! Les règles d'achat rappelées sont celles que kreek code : leurs chiffres
//! viennent des constantes du domaine et des use cases, pas du gabarit.

use crate::app::match_report::domain::match_report_pre_match::{
    MAX_MERCENARIES, MAX_STAR_PLAYERS, MAX_TREASURY_TOP_UP,
};
use crate::app::match_report::use_cases::inducements_print_service::{
    self, InducementsCatalog, StarOffer, TeamCatalog,
};
use crate::app::match_report::use_cases::init_temp_players_use_case::JOURNEYMEN_FILL_TO;
use crate::app::match_report::use_cases::record_inducements_use_case::MercenaryLevel;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Normal,
    Compact,
}

#[derive(Debug, Default, Deserialize)]
pub struct InducementsPrintQuery {
    #[serde(default)]
    pub inducements: bool,
    #[serde(default)]
    pub star_players: bool,
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub density: Density,
}

impl InducementsPrintQuery {
    /// Fiches de star player par feuille ; la première en perd quand le tableau
    /// des coups de pouce la partage.
    fn stars_per_page(&self) -> usize {
        match (self.orientation, self.density) {
            (Orientation::Landscape, _) => 6,
            (Orientation::Portrait, Density::Compact) => 10,
            (Orientation::Portrait, Density::Normal) => 8,
        }
    }

    fn stars_on_first_page(&self) -> usize {
        if !self.inducements {
            return self.stars_per_page();
        }
        let taken = if self.orientation == Orientation::Landscape {
            3
        } else {
            2
        };
        self.stars_per_page() - taken
    }
}

pub struct InducementRowVm {
    pub name: String,
    pub description: String,
    /// Un prix par équipe imprimée ; `None` : non disponible pour elle (« — »).
    pub prices: Vec<Option<u32>>,
    pub max_qty: u8,
}

pub struct StarCardVm {
    pub name: String,
    pub player_type: String,
    pub cost_kpo: u32,
    pub stats: [String; 5],
    pub skills: String,
    pub ability_name: String,
    pub ability_description: String,
    pub teams: String,
}

/// Les règles d'achat que kreek code, rappelées avec leurs chiffres.
pub struct PurchaseRulesVm {
    pub treasury_top_up_kpo: u32,
    pub max_star_players: usize,
    pub mercenary_base_kpo: u32,
    pub mercenary_lvl1_kpo: u32,
    pub max_mercenaries: u8,
    pub journeymen_fill_to: usize,
}

pub struct InducementsPageVm {
    pub with_inducements: bool,
    pub stars: Vec<StarCardVm>,
    pub stars_heading: bool,
}

pub struct InducementsPrintVm {
    pub team_names: Vec<String>,
    pub missing_tiers: Vec<String>,
    pub rows: Vec<InducementRowVm>,
    pub rules: PurchaseRulesVm,
    pub total_stars: usize,
    pub pages: Vec<InducementsPageVm>,
}

impl InducementsPrintVm {
    pub fn build(catalog: InducementsCatalog, query: &InducementsPrintQuery) -> Self {
        let rows = catalog
            .labels
            .iter()
            .map(|l| inducement_row(&catalog.teams, &l.uid, &l.name, &l.description))
            .collect();
        let stars: Vec<StarCardVm> = if query.star_players {
            catalog.stars.into_iter().map(star_card).collect()
        } else {
            Vec::new()
        };
        Self {
            team_names: catalog.teams.iter().map(|t| t.team_name.clone()).collect(),
            missing_tiers: catalog
                .teams
                .iter()
                .filter(|t| t.tier.is_none())
                .map(|t| t.team_name.clone())
                .collect(),
            rows,
            rules: purchase_rules(),
            total_stars: stars.len(),
            pages: paginate(stars, query),
        }
    }
}

fn inducement_row(
    teams: &[TeamCatalog],
    uid: &str,
    name: &str,
    description: &str,
) -> InducementRowVm {
    let spec = |t: &TeamCatalog| {
        t.tier
            .as_ref()?
            .allowed_inducements
            .iter()
            .find(|s| s.uid == uid)
            .cloned()
    };
    InducementRowVm {
        name: name.to_string(),
        description: description.to_string(),
        prices: teams.iter().map(|t| spec(t).map(|s| s.unit_cost)).collect(),
        max_qty: teams.iter().find_map(spec).map(|s| s.max_qty).unwrap_or(0),
    }
}

fn star_card(offer: StarOffer) -> StarCardVm {
    let s = offer.star;
    StarCardVm {
        name: s.name,
        player_type: s.player_type,
        cost_kpo: s.cost,
        stats: [s.ma.to_string(), s.st.to_string(), s.ag, s.pa, s.av],
        skills: s.skills.join(", "),
        ability_name: s.ability_name,
        ability_description: s.ability_description,
        teams: offer.team_names.join(" et "),
    }
}

fn purchase_rules() -> PurchaseRulesVm {
    PurchaseRulesVm {
        treasury_top_up_kpo: MAX_TREASURY_TOP_UP,
        max_star_players: MAX_STAR_PLAYERS,
        mercenary_base_kpo: MercenaryLevel::Base.extra_cost(),
        mercenary_lvl1_kpo: MercenaryLevel::Lvl1.extra_cost(),
        max_mercenaries: MAX_MERCENARIES,
        journeymen_fill_to: JOURNEYMEN_FILL_TO,
    }
}

/// Le tableau sur la première feuille, les fiches ensuite, autant de feuilles
/// qu'il en faut.
fn paginate(mut stars: Vec<StarCardVm>, query: &InducementsPrintQuery) -> Vec<InducementsPageVm> {
    let mut pages = Vec::new();
    let first = stars
        .drain(..query.stars_on_first_page().min(stars.len()))
        .collect::<Vec<_>>();
    if query.inducements || !first.is_empty() {
        pages.push(InducementsPageVm {
            with_inducements: query.inducements,
            stars_heading: !first.is_empty(),
            stars: first,
        });
    }
    while !stars.is_empty() {
        let page = stars
            .drain(..query.stars_per_page().min(stars.len()))
            .collect();
        pages.push(InducementsPageVm {
            with_inducements: false,
            stars_heading: false,
            stars: page,
        });
    }
    pages
}

#[derive(Template)]
#[template(path = "match-report-print-inducements.html")]
pub struct InducementsPrintTemplate {
    pub vm: InducementsPrintVm,
}

impl IntoResponse for InducementsPrintTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("match report print inducements render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

async fn rendre(state: &AppState, team_ids: &[&str], query: &InducementsPrintQuery) -> Response {
    let ctx = &state.match_report;
    let built = inducements_print_service::build(
        ctx.team_data.as_ref(),
        ctx.competition_data.as_ref(),
        ctx.reference_catalog.as_ref(),
        team_ids,
    )
    .await;
    match built {
        Ok(catalog) => InducementsPrintTemplate {
            vm: InducementsPrintVm::build(catalog, query),
        }
        .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn get_print_inducements(
    Path((_space_id, team_id)): Path<(String, String)>,
    Query(query): Query<InducementsPrintQuery>,
    State(state): State<AppState>,
) -> Response {
    rendre(&state, &[&team_id], &query).await
}

/// `{other_team_id}` est contrôlé par `space_scope` (carte 594).
pub async fn get_print_inducements_with(
    Path((_space_id, team_id, other_team_id)): Path<(String, String, String)>,
    Query(query): Query<InducementsPrintQuery>,
    State(state): State<AppState>,
) -> Response {
    rendre(&state, &[&team_id, &other_team_id], &query).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::ports::{InducementSpecDto, TierRulesDto};

    fn equipe(nom: &str, specs: &[(&str, u32)]) -> TeamCatalog {
        TeamCatalog {
            team_name: nom.into(),
            tier: Some(TierRulesDto {
                allowed_inducements: specs
                    .iter()
                    .map(|(uid, cost)| InducementSpecDto {
                        uid: uid.to_string(),
                        max_qty: 2,
                        unit_cost: *cost,
                    })
                    .collect(),
                allowed_star_players: vec![],
            }),
        }
    }

    fn carte(i: usize) -> StarCardVm {
        StarCardVm {
            name: format!("Étoile {i}"),
            player_type: String::new(),
            cost_kpo: 100,
            stats: Default::default(),
            skills: String::new(),
            ability_name: String::new(),
            ability_description: String::new(),
            teams: String::new(),
        }
    }

    /// R7 : un prix par équipe, « — » quand le coup de pouce lui est fermé.
    #[test]
    fn chaque_equipe_a_son_prix_ou_rien() {
        let teams = [
            equipe("A", &[("MASSEUR", 30)]),
            equipe("B", &[("CHARIOT", 15)]),
        ];
        let ligne = inducement_row(&teams, "MASSEUR", "Masseur", "");
        assert_eq!(ligne.prices, vec![Some(30), None]);
        assert_eq!(ligne.max_qty, 2);
    }

    #[test]
    fn le_tableau_partage_la_premiere_feuille_avec_les_fiches() {
        let query = InducementsPrintQuery {
            inducements: true,
            star_players: true,
            ..Default::default()
        };
        let pages = paginate((0..9).map(carte).collect(), &query);
        let par_page: Vec<usize> = pages.iter().map(|p| p.stars.len()).collect();
        assert_eq!(par_page, vec![6, 3]);
        assert!(pages[0].with_inducements && !pages[1].with_inducements);
    }

    #[test]
    fn sans_les_coups_de_pouce_les_fiches_prennent_toute_la_feuille() {
        let query = InducementsPrintQuery {
            star_players: true,
            ..Default::default()
        };
        let pages = paginate((0..9).map(carte).collect(), &query);
        assert_eq!(
            pages.iter().map(|p| p.stars.len()).collect::<Vec<_>>(),
            vec![8, 1]
        );
    }

    #[test]
    fn les_regles_rappelees_sont_celles_du_code() {
        let r = purchase_rules();
        assert_eq!(
            (r.treasury_top_up_kpo, r.max_star_players, r.max_mercenaries),
            (50, 2, 3)
        );
        assert_eq!(
            (
                r.mercenary_base_kpo,
                r.mercenary_lvl1_kpo,
                r.journeymen_fill_to
            ),
            (30, 80, 11)
        );
    }
}
