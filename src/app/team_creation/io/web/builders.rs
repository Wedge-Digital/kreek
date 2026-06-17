use crate::app::team_creation::domain::creation_rules::CreationRules;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::io::web::view_models::{
    HiredPlayerRowVm, PlayerPositionVm, RosterPickerItemWithTier,
};
use crate::app::team_creation::ports::{IReferenceDataPort, RosterDefinition};

fn to_stat_plus(v: u8) -> String {
    if v == 0 {
        "—".into()
    } else {
        format!("{}+", v)
    }
}

pub fn build_player_positions(roster_def: &RosterDefinition) -> Vec<PlayerPositionVm> {
    roster_def
        .available_players
        .iter()
        .map(|p| {
            let skills = p
                .skills
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            PlayerPositionVm {
                uid: p.uid.clone(),
                name: p.position_name.clone(),
                cost: p.cost / 1000,
                max_qty_label: format!("0-{}", p.max_quantity),
                ma: p.ma,
                st: p.st,
                ag: to_stat_plus(p.ag),
                pa: to_stat_plus(p.pa),
                av: to_stat_plus(p.av),
                skills,
            }
        })
        .collect()
}

pub fn build_hired_rows(
    team: &RosterSelectedTeam,
    roster_def: &RosterDefinition,
) -> Vec<HiredPlayerRowVm> {
    team.roster
        .player_definitions
        .iter()
        .map(|def| {
            let quantity = team
                .hired_players()
                .iter()
                .filter(|p| p.definition.id == def.id)
                .count();
            let line_cost_kpo = quantity as u32 * def.price.0;
            let is_max =
                quantity >= def.max_quantity.0 as usize || team.hired_players().len() >= 16;

            let pos = roster_def
                .available_players
                .iter()
                .find(|p| p.uid == def.id.0);

            let (ma, st, ag, pa, av, skills) = match pos {
                Some(p) => {
                    let skills = p
                        .skills
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    (
                        p.ma,
                        p.st,
                        to_stat_plus(p.ag),
                        to_stat_plus(p.pa),
                        to_stat_plus(p.av),
                        skills,
                    )
                }
                None => (0, 0, "?".into(), "?".into(), "?".into(), String::new()),
            };

            HiredPlayerRowVm {
                uid: def.id.0.clone(),
                name: def.name.0.clone(),
                cost_kpo: def.price.0,
                max_qty_label: format!("0-{}", def.max_quantity.0),
                ma,
                st,
                ag,
                pa,
                av,
                skills,
                quantity,
                line_cost_kpo,
                is_max,
            }
        })
        .collect()
}

pub fn build_roster_items_with_tiers(
    ref_data: &dyn IReferenceDataPort,
    rules: &CreationRules,
) -> Vec<RosterPickerItemWithTier> {
    let mut items: Vec<RosterPickerItemWithTier> = rules
        .tiers
        .iter()
        .enumerate()
        .flat_map(|(i, tier)| {
            let tier_name = tier.name.clone();
            let tier_index = i + 1;
            tier.rosters.iter().filter_map(move |uid| {
                ref_data
                    .find_roster_definition(uid)
                    .map(|def| RosterPickerItemWithTier {
                        uid: def.uid,
                        name: def.name,
                        tier_name: tier_name.clone(),
                        tier_index,
                        reroll_cost: def.reroll_cost,
                    })
            })
        })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}
