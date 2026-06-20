use crate::app::team_creation::domain::team_roster_selected::{
    RosterSelectedTeam, MAX_REROLL_COUNT,
};
use crate::app::team_creation::domain::team_staff::TeamStaff;

// ── Staff ────────────────────────────────────────────────────────────────────

pub struct StaffRowVm {
    pub id: String,
    pub name: String,
    pub price: u32,
    pub max_qty_label: String,
    pub max_qty: usize,
    pub quantity: usize,
    pub line_cost: u32,
}

impl StaffRowVm {
    pub fn from_domain(staff: &TeamStaff, hired_count: usize) -> Self {
        Self {
            id: staff.id.0.clone(),
            name: staff.name.0.clone(),
            price: staff.price.0,
            max_qty_label: format!("0-{}", staff.max_quantity.0),
            max_qty: staff.max_quantity.0 as usize,
            quantity: hired_count,
            line_cost: hired_count as u32 * staff.price.0,
        }
    }

    pub fn all_from_domain(team: &RosterSelectedTeam) -> Vec<Self> {
        team.roster
            .allowed_staff
            .iter()
            .map(|staff| {
                let hired_count = team
                    .hired_staff()
                    .iter()
                    .filter(|s| s.id == staff.id)
                    .count();
                Self::from_domain(staff, hired_count)
            })
            .collect()
    }
}

// ── Reroll ───────────────────────────────────────────────────────────────────

pub struct RerollVm {
    pub price: u32,
    pub quantity: u8,
    pub max: u8,
    pub line_cost: u32,
}

impl RerollVm {
    pub fn from_domain(team: &RosterSelectedTeam) -> Self {
        let price = team.roster.reroll_price.0;
        let quantity = team.reroll_count();
        Self {
            price,
            quantity,
            max: MAX_REROLL_COUNT,
            line_cost: price * quantity as u32,
        }
    }
}

// ── Cart ─────────────────────────────────────────────────────────────────────

pub struct CartLineVm {
    pub name: String,
    pub qty: usize,
    pub total: u32,
}

pub struct CartVm {
    pub player_lines: Vec<CartLineVm>,
    pub player_subtotal: u32,
    pub reroll_qty: u8,
    pub reroll_cost: u32,
    pub staff_lines: Vec<CartLineVm>,
    pub staff_subtotal: u32,
    pub total: u32,
    pub budget: u32,
    pub remaining: u32,
    pub progress_pct: u32,
    pub player_count: usize,
    pub min_players: usize,
}

impl CartVm {
    pub fn from_domain(team: &RosterSelectedTeam) -> Self {
        let mut player_lines: Vec<CartLineVm> = Vec::new();
        for def in &team.roster.player_definitions {
            let qty = team
                .hired_players()
                .iter()
                .filter(|p| p.definition.id == def.id)
                .count();
            if qty > 0 {
                player_lines.push(CartLineVm {
                    name: def.name.0.clone(),
                    qty,
                    total: qty as u32 * def.price.0,
                });
            }
        }
        let player_subtotal: u32 = player_lines.iter().map(|l| l.total).sum();

        let reroll_qty = team.reroll_count();
        let reroll_cost = team.roster.reroll_price.0 * reroll_qty as u32;

        let mut staff_lines: Vec<CartLineVm> = Vec::new();
        for staff in &team.roster.allowed_staff {
            let qty = team
                .hired_staff()
                .iter()
                .filter(|s| s.id == staff.id)
                .count();
            if qty > 0 {
                staff_lines.push(CartLineVm {
                    name: staff.name.0.clone(),
                    qty,
                    total: qty as u32 * staff.price.0,
                });
            }
        }
        let staff_subtotal: u32 = staff_lines.iter().map(|l| l.total).sum();

        let total = player_subtotal + staff_subtotal + reroll_cost;
        let remaining = team.remaining_budget().unwrap_or(0);
        let budget = total + remaining;
        let progress_pct = if budget > 0 {
            (total * 100 / budget).min(100)
        } else {
            0
        };

        Self {
            player_lines,
            player_subtotal,
            reroll_qty,
            reroll_cost,
            staff_lines,
            staff_subtotal,
            total,
            budget,
            remaining,
            progress_pct,
            player_count: team.hired_players().len(),
            min_players: crate::app::team_creation::domain::team_roster_selected::MIN_PLAYERS_FOR_SUBMISSION,
        }
    }
}

// ── VMs dépendant du port (construits par builders.rs) ───────────────────────

pub struct HiredPlayerRowVm {
    pub uid: String,
    pub name: String,
    pub cost_kpo: u32,
    pub max_qty_label: String,
    pub ma: u8,
    pub st: u8,
    pub ag: String,
    pub pa: String,
    pub av: String,
    pub skills: String,
    pub quantity: usize,
    pub line_cost_kpo: u32,
    pub is_max: bool,
}

pub struct PlayerPositionVm {
    pub uid: String,
    pub name: String,
    pub cost: u32,
    pub max_qty_label: String,
    pub ma: u8,
    pub st: u8,
    pub ag: String,
    pub pa: String,
    pub av: String,
    pub skills: String,
}

pub struct RosterPickerItemWithTier {
    pub uid: String,
    pub name: String,
    pub tier_name: String,
    pub tier_index: usize,
    pub reroll_cost: u32,
}

// ── Rules panel ──────────────────────────────────────────────────────────────

pub struct RulesTierVm {
    pub name: String,
    pub budget: u32,
    pub start_xp: u32,
}

pub struct RulesPanelVm {
    pub competition_name: String,
    pub season_name: String,
    pub tiers: Vec<RulesTierVm>,
}

// ── Finalize page VMs ────────────────────────────────────────────────────────

pub struct FinalizePlayerVm {
    pub id: String,
    pub jersey: u8,
    pub name: String,
    pub position_name: String,
    pub roster_line_id: String,
    pub base_skills: Vec<String>,
    pub acquired_count: usize,
    pub acquired_csv: String,
}

pub struct SppLogEntryVm {
    pub player_id: String,
    pub player_name: String,
    pub jersey: u8,
    pub position_name: String,
    pub skill_id: String,
    pub skill_name: String,
    pub mode_label: String,
    pub spp_cost: u8,
}
