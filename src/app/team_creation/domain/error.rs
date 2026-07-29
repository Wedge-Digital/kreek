use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    NoRuleset,
    RosterNotAllowed,
    RosterNotInRuleset,
    MaxPlayersReached,
    MaxPlayersOfTypeReached,
    PlayerNotAllowedInRoster,
    CrossLimitExceeded,
    InsufficientBudget,
    PlayerNotHired,
    StaffNotPurchased,
    StaffNotAllowed,
    StaffMaxReached,
    MaxRerollsExceeded,
    InsufficientRerollBudget,
    PlayerNotFoundInTeam,
    StaffNotFoundInTeam,
    InsufficientPlayerCount,
    PlayerNameTooLong,
    DuplicateJerseyNumber,
    LeagueNotSelected,
    InsufficientSpp,
    SkillAlreadyAcquired,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            DomainError::NoRuleset => "team_creation.no_ruleset",
            DomainError::RosterNotAllowed => "team_creation.roster_not_allowed",
            DomainError::RosterNotInRuleset => "team_creation.roster_not_present_in_ruleset",
            DomainError::MaxPlayersReached => "team_creation.max_players_reached",
            DomainError::MaxPlayersOfTypeReached => "team_creation.max_players_of_type_reached",
            DomainError::PlayerNotAllowedInRoster => "team_creation.hire_player_impossible",
            DomainError::CrossLimitExceeded => "team_creation.cross_limit_failed",
            DomainError::InsufficientBudget => "team_creation.insufficient_budget",
            DomainError::PlayerNotHired => "team_creation.player_not_hired",
            DomainError::StaffNotPurchased => "team_creation.staff_not_purchased",
            DomainError::StaffNotAllowed => "team_creation.staff_not_allowed",
            DomainError::StaffMaxReached => "team_creation.staff_max_reached",
            DomainError::MaxRerollsExceeded => "team_creation.max_rr_exceeded",
            DomainError::InsufficientRerollBudget => "team_creation.insufficient_reroll_budget",
            DomainError::PlayerNotFoundInTeam => "team_creation.player_not_found_in_team",
            DomainError::StaffNotFoundInTeam => "team_creation.staff_not_found_in_team",
            DomainError::InsufficientPlayerCount => "team_creation.insufficient_player_count",
            DomainError::PlayerNameTooLong => "Le nom ne peut pas dépasser 50 caractères.",
            DomainError::DuplicateJerseyNumber => {
                "Ce numéro de maillot est déjà attribué à un autre joueur."
            }
            DomainError::LeagueNotSelected => "Veuillez sélectionner une ligue avant de soumettre.",
            DomainError::InsufficientSpp => "Budget SPP insuffisant.",
            DomainError::SkillAlreadyAcquired => "Ce joueur possède déjà cette compétence.",
        };
        write!(f, "{}", msg)
    }
}
