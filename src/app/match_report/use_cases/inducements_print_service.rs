//! Les coups de pouce et star players d'une impression (carte 607).
//!
//! Pour chaque équipe imprimée : le palier de **sa** compétition, au prix de
//! **son** roster (R7) — ce que `find_tier_rules_for_roster` calcule déjà pour
//! le rapport de match, disponibilités comprises. Puis les libellés et les
//! fiches des star players, lus dans le catalogue.

use crate::app::match_report::ports::{
    ICompetitionDataPort, IReferenceCatalogPort, ITeamDataPort, InducementLabelDto, StarPlayerDto,
    TierRulesDto,
};

#[derive(Debug, PartialEq, Eq)]
pub enum PrintError {
    TeamNotFound,
}

/// Le catalogue d'une équipe. `tier` vaut `None` sans saison ou sans palier :
/// la colonne s'imprime vide et le dit, plutôt qu'un catalogue inventé.
pub struct TeamCatalog {
    pub team_name: String,
    pub tier: Option<TierRulesDto>,
}

/// Un star player, et les équipes imprimées qui peuvent l'engager.
pub struct StarOffer {
    pub star: StarPlayerDto,
    pub team_names: Vec<String>,
}

pub struct InducementsCatalog {
    pub teams: Vec<TeamCatalog>,
    pub labels: Vec<InducementLabelDto>,
    pub stars: Vec<StarOffer>,
}

// arch:no-instrument — service de lecture : assemble le catalogue imprimé, sans intention métier
pub async fn build(
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
    catalog: &dyn IReferenceCatalogPort,
    team_ids: &[&str],
) -> Result<InducementsCatalog, PrintError> {
    let mut teams = Vec::new();
    let mut stars: Vec<StarOffer> = Vec::new();
    for team_id in team_ids {
        let (team, roster_id) = team_catalog(team_data, competition_data, team_id).await?;
        add_stars(&mut stars, catalog, &team, &roster_id);
        teams.push(team);
    }
    let labels = catalog.find_inducement_labels(&inducement_uids(&teams));
    Ok(InducementsCatalog {
        teams,
        labels,
        stars,
    })
}

async fn team_catalog(
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
    team_id: &str,
) -> Result<(TeamCatalog, String), PrintError> {
    let info = team_data
        .find_team_info(team_id)
        .await
        .ok_or(PrintError::TeamNotFound)?;
    let tier = match team_data.find_team_season(team_id).await {
        Some(season) => {
            competition_data
                .find_tier_rules_for_roster(&season, &info.roster_id)
                .await
        }
        None => None,
    };
    let team = TeamCatalog {
        team_name: info.team_name,
        tier,
    };
    Ok((team, info.roster_id))
}

/// Les coups de pouce de tous les paliers, chacun une fois, dans l'ordre où
/// ils apparaissent.
fn inducement_uids(teams: &[TeamCatalog]) -> Vec<String> {
    let mut uids: Vec<String> = Vec::new();
    for spec in teams
        .iter()
        .flat_map(|t| t.tier.iter())
        .flat_map(|t| &t.allowed_inducements)
    {
        if !uids.contains(&spec.uid) {
            uids.push(spec.uid.clone());
        }
    }
    uids
}

/// Les star players que cette équipe peut engager (R13), fusionnés avec ceux
/// des équipes déjà vues.
fn add_stars(
    stars: &mut Vec<StarOffer>,
    catalog: &dyn IReferenceCatalogPort,
    team: &TeamCatalog,
    roster_id: &str,
) {
    let Some(tier) = &team.tier else { return };
    let uids: Vec<String> = tier
        .allowed_star_players
        .iter()
        .map(|s| s.uid.clone())
        .collect();
    for star in catalog.find_star_players(&uids, roster_id) {
        match stars.iter_mut().find(|o| o.star.uid == star.uid) {
            Some(offer) => offer.team_names.push(team.team_name.clone()),
            None => stars.push(StarOffer {
                star,
                team_names: vec![team.team_name.clone()],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::ports::InducementSpecDto;

    fn spec(uid: &str, cost: u32) -> InducementSpecDto {
        InducementSpecDto {
            uid: uid.into(),
            max_qty: 1,
            unit_cost: cost,
        }
    }

    fn equipe(nom: &str, uids: &[&str]) -> TeamCatalog {
        TeamCatalog {
            team_name: nom.into(),
            tier: Some(TierRulesDto {
                allowed_inducements: uids.iter().map(|u| spec(u, 10)).collect(),
                allowed_star_players: vec![],
            }),
        }
    }

    #[test]
    fn les_coups_de_pouce_de_deux_paliers_une_fois_chacun() {
        let teams = [equipe("A", &["X", "Y"]), equipe("B", &["Y", "Z"])];
        assert_eq!(inducement_uids(&teams), vec!["X", "Y", "Z"]);
    }

    #[test]
    fn une_equipe_sans_palier_n_ajoute_rien() {
        let teams = [
            TeamCatalog {
                team_name: "Sans".into(),
                tier: None,
            },
            equipe("A", &["X"]),
        ];
        assert_eq!(inducement_uids(&teams), vec!["X"]);
    }
}
