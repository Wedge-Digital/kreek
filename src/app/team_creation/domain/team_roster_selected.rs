use crate::app::shared_kernel::common_types::Entity;
use crate::app::shared_kernel::staff::StaffKind;
use crate::app::shared_kernel::team::TeamId;
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::{PlayerDefinition, Roster, MAX_PLAYER_COUNT};
use crate::app::team_creation::domain::team_ruleset_selected::RulesetSelectedTeam;
use crate::app::team_creation::domain::team_staff::TeamStaff;
use serde::{Deserialize, Serialize};

pub const MAX_REROLL_COUNT:           u8    = 8;
pub const MIN_PLAYERS_FOR_SUBMISSION: usize = 11;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RosterSelectedTeam {
    team: RulesetSelectedTeam,
    pub roster: Roster,
    hired_players: Vec<PlayerDefinition>,
    hired_staff: Vec<TeamStaff>,
    reroll_count: u8,
}

impl RosterSelectedTeam {
    pub fn new(team: RulesetSelectedTeam, roster: Roster) -> Self {
        RosterSelectedTeam {
            team,
            roster,
            hired_players: Vec::new(),
            hired_staff: Vec::new(),
            reroll_count: 0,
        }
    }

    pub fn hired_players(&self) -> &[PlayerDefinition] {
        &self.hired_players
    }

    pub fn hired_staff(&self) -> &[TeamStaff] {
        &self.hired_staff
    }

    pub fn reroll_count(&self) -> u8 {
        self.reroll_count
    }

    pub fn cheerleaders_count(&self) -> usize {
        self.hired_staff.iter().filter(|s| s.kind == StaffKind::Cheerleaders).count()
    }

    fn player_budget(&self) -> u32 {
        self.hired_players.iter().map(|p| p.price.0).sum()
    }

    fn staff_budget(&self) -> u32 {
        self.hired_staff.iter().map(|s| s.price.0).sum()
    }

    fn reroll_budget(&self) -> u32 {
        self.reroll_count as u32 * self.roster.reroll_price.0
    }

    pub fn remaining_budget(&self) -> Result<u32, DomainError> {
        let budget = self.team.ruleset.get_creation_budget(&self.roster.id)?;
        let spent = self.player_budget() + self.staff_budget() + self.reroll_budget();
        Ok(budget.0.saturating_sub(spent))
    }

    // -------------------------------------------------------------------------
    // Changement de roster (réinitialise les achats si roster différent)
    // -------------------------------------------------------------------------

    pub fn choose_roster(self, roster: Roster) -> Result<RosterSelectedTeam, DomainError> {
        if self.team.ruleset.is_roster_not_allowed(&roster.id) {
            return Err(DomainError::RosterNotAllowed);
        }
        if roster.id == self.roster.id {
            return Ok(self);
        }
        Ok(RosterSelectedTeam::new(self.team, roster))
    }

    // -------------------------------------------------------------------------
    // Joueurs
    // -------------------------------------------------------------------------

    fn check_player_is_allowed(&self, player: &PlayerDefinition) -> Result<(), DomainError> {
        if !self.roster.contains_player(player) {
            return Err(DomainError::PlayerNotAllowedInRoster);
        }
        Ok(())
    }

    fn check_max_total_players(&self) -> Result<(), DomainError> {
        if self.hired_players.len() >= MAX_PLAYER_COUNT as usize {
            return Err(DomainError::MaxPlayersReached);
        }
        Ok(())
    }

    fn check_max_players_of_type(&self, player: &PlayerDefinition) -> Result<(), DomainError> {
        let count = self.hired_players.iter().filter(|p| *p == player).count();
        if count >= player.max_quantity.0 as usize {
            return Err(DomainError::MaxPlayersOfTypeReached);
        }
        Ok(())
    }

    fn check_cross_limits(&self, player: &PlayerDefinition) -> Result<(), DomainError> {
        if !self.roster.has_cross_limits() {
            return Ok(());
        }
        for limit in &self.roster.cross_limits {
            if !limit.includes_player(&player.id) {
                continue;
            }
            let count = self.hired_players.iter()
                .filter(|p| limit.includes_player(&p.id))
                .count();
            if count >= limit.limit as usize {
                return Err(DomainError::CrossLimitExceeded);
            }
        }
        Ok(())
    }

    fn check_player_budget(&self, player: &PlayerDefinition) -> Result<(), DomainError> {
        let remaining = self.remaining_budget().map_err(|_| DomainError::InsufficientBudget)?;
        if remaining < player.price.0 {
            return Err(DomainError::InsufficientBudget);
        }
        Ok(())
    }

    pub fn hire_player(&mut self, player: PlayerDefinition) -> Result<(), DomainError> {
        self.check_player_is_allowed(&player)
            .and_then(|_| self.check_max_total_players())
            .and_then(|_| self.check_max_players_of_type(&player))
            .and_then(|_| self.check_cross_limits(&player))
            .and_then(|_| self.check_player_budget(&player))?;

        self.hired_players.push(player);
        Ok(())
    }

    pub fn hire_many_players(&mut self, players: Vec<PlayerDefinition>) -> Result<(), Vec<DomainError>> {
        let errors: Vec<DomainError> = players.into_iter()
            .flat_map(|p| self.hire_player(p).err())
            .collect();
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    pub fn remove_player(&mut self, player: &PlayerDefinition) -> Result<(), DomainError> {
        let pos = self.hired_players.iter().position(|p| p == player)
            .ok_or(DomainError::PlayerNotHired)?;
        self.hired_players.remove(pos);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Staff
    // -------------------------------------------------------------------------

    fn check_staff_is_allowed(&self, staff: &TeamStaff) -> Result<(), DomainError> {
        if self.roster.allowed_staff.contains(staff) {
            return Ok(());
        }
        Err(DomainError::StaffNotAllowed)
    }

    fn check_staff_limit(&self, staff: &TeamStaff) -> Result<(), DomainError> {
        let count = self.hired_staff.iter().filter(|s| *s == staff).count();
        if count >= staff.max_quantity.0 as usize {
            return Err(DomainError::StaffMaxReached);
        }
        Ok(())
    }

    fn check_staff_budget(&self, staff: &TeamStaff) -> Result<(), DomainError> {
        let remaining = self.remaining_budget().map_err(|_| DomainError::InsufficientBudget)?;
        if remaining < staff.price.0 {
            return Err(DomainError::InsufficientBudget);
        }
        Ok(())
    }

    pub fn buy_staff(&mut self, staff: TeamStaff) -> Result<(), Vec<DomainError>> {
        let mut errors: Vec<DomainError> = Vec::new();
        if let Err(e) = self.check_staff_limit(&staff) { errors.push(e); }
        if let Err(e) = self.check_staff_budget(&staff) { errors.push(e); }
        if let Err(e) = self.check_staff_is_allowed(&staff) { errors.push(e); }
        if !errors.is_empty() {
            return Err(errors);
        }
        self.hired_staff.push(staff);
        Ok(())
    }

    pub fn remove_staff(&mut self, staff: &TeamStaff) -> Result<(), DomainError> {
        let pos = self.hired_staff.iter().position(|s| s == staff)
            .ok_or(DomainError::StaffNotPurchased)?;
        self.hired_staff.remove(pos);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Rerolls
    // -------------------------------------------------------------------------

    fn check_reroll_max(&self, count: u8) -> Result<(), DomainError> {
        if self.reroll_count.saturating_add(count) > MAX_REROLL_COUNT {
            return Err(DomainError::MaxRerollsExceeded);
        }
        Ok(())
    }

    fn check_reroll_budget(&self, count: u8) -> Result<(), DomainError> {
        let cost = count as u32 * self.roster.reroll_price.0;
        let remaining = self.remaining_budget().map_err(|_| DomainError::InsufficientRerollBudget)?;
        if remaining < cost {
            return Err(DomainError::InsufficientRerollBudget);
        }
        Ok(())
    }

    pub fn purchase_reroll(&mut self, count: u8) -> Result<(), Vec<DomainError>> {
        let mut errors: Vec<DomainError> = Vec::new();
        if let Err(e) = self.check_reroll_max(count) { errors.push(e); }
        if let Err(e) = self.check_reroll_budget(count) { errors.push(e); }
        if !errors.is_empty() {
            return Err(errors);
        }
        self.reroll_count += count;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Validation de soumission
    // -------------------------------------------------------------------------

    pub fn validate_for_submission(&self) -> Result<(), Vec<DomainError>> {
        let mut errors = Vec::new();
        if self.hired_players.len() < MIN_PLAYERS_FOR_SUBMISSION {
            errors.push(DomainError::InsufficientPlayerCount);
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    pub fn remove_reroll(&mut self, count: u8) {
        if count >= self.reroll_count {
            self.reroll_count = 0;
        } else {
            self.reroll_count -= count;
        }
    }
}

impl PartialEq for RosterSelectedTeam {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl Entity for RosterSelectedTeam {
    fn get_id(&self) -> TeamId {
        self.team.get_id()
    }

    fn get_created_by(&self) -> TeamId {
        self.team.get_created_by()
    }
}