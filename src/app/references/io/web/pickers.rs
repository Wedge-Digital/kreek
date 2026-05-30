use crate::app::references::domain::models::Team;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::team_creation::domain::creation_rules::CreationRules;

pub struct RosterPickerItem {
    pub uid:  String,
    pub name: String,
}

pub struct RosterPickerItemWithTier {
    pub uid:         String,
    pub name:        String,
    pub tier_name:   String,  // nom du tier dans la compétition (ex. "Débutants")
    pub tier_index:  usize,   // position 1-based du tier dans les règles (pour la classe CSS)
    pub reroll_cost: u32,
}

pub struct InducementPickerItem {
    pub uid:  String,
    pub name: String,
    pub cost: u32,
}

pub struct StarPlayerPickerItem {
    pub uid:  String,
    pub name: String,
}

pub fn build_roster_items_with_tiers(
    repo:  &dyn IReferenceRepository,
    rules: &CreationRules,
) -> Vec<RosterPickerItemWithTier> {
    let mut items: Vec<RosterPickerItemWithTier> = rules.tiers.iter()
        .enumerate()
        .flat_map(|(i, tier)| {
            let tier_name  = tier.name.clone();
            let tier_index = i + 1;
            tier.rosters.iter().filter_map(move |uid| {
                repo.find_team_by_uid(uid).map(|t| RosterPickerItemWithTier {
                    uid:        t.uid.clone(),
                    name:       t.name.clone(),
                    tier_name:  tier_name.clone(),
                    tier_index,
                    reroll_cost: t.reroll_cost,
                })
            })
        })
        .collect();

    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

pub struct PlayerPositionVm {
    pub name:          String,
    pub cost:          u32,
    pub max_qty_label: String,
    pub ma:            u8,
    pub st:            u8,
    pub ag:            String,
    pub pa:            String,
    pub av:            String,
    pub skills:        String,
}

fn to_stat_plus(v: u8) -> String {
    if v == 0 { "—".into() } else { format!("{}+", v) }
}

pub fn build_player_positions(team: &Team, repo: &dyn IReferenceRepository) -> Vec<PlayerPositionVm> {
    team.available_players.iter().map(|p| {
        let skills = p.skills.iter()
            .map(|uid| repo.find_skill_by_uid(uid)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| uid.clone()))
            .collect::<Vec<_>>()
            .join(", ");
        PlayerPositionVm {
            name:          p.position_name.clone(),
            cost:          p.cost,
            max_qty_label: format!("0-{}", p.max_quantity),
            ma:            p.ma,
            st:            p.st,
            ag:            to_stat_plus(p.ag),
            pa:            to_stat_plus(p.pa),
            av:            to_stat_plus(p.av),
            skills,
        }
    }).collect()
}

pub fn build_roster_items(repo: &dyn IReferenceRepository) -> Vec<RosterPickerItem> {
    let mut items: Vec<RosterPickerItem> = repo.list_teams()
        .iter()
        .map(|t| RosterPickerItem { uid: t.uid.clone(), name: t.name.clone() })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

pub fn build_inducement_items(repo: &dyn IReferenceRepository) -> Vec<InducementPickerItem> {
    repo.list_inducements()
        .iter()
        .map(|i| InducementPickerItem { uid: i.uid.clone(), name: i.name.clone(), cost: i.cost })
        .collect()
}

pub fn build_star_player_items(repo: &dyn IReferenceRepository) -> Vec<StarPlayerPickerItem> {
    repo.list_star_players()
        .iter()
        .map(|s| StarPlayerPickerItem { uid: s.uid.clone(), name: s.name.clone() })
        .collect()
}
