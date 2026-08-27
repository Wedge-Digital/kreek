pub mod path {
    pub const PLAYERS_BY_TEAM_WIDGET: &str = "/app/{space_id}/players/by-team/{team_id}/widget";
    pub const PLAYERS_ROSTER_UPDATE: &str = "/app/{space_id}/players/by-team/{team_id}/roster";
    pub const MATCH_PLAYER_SELECTOR: &str =
        "/app/{space_id}/players/teams/{team_id}/match-selector";
    pub const PLAYER_DEBUG: &str = "/app/{space_id}/players/{player_id}/debug";
    pub const PLAYER_DETAIL: &str = "/app/{space_id}/players/{player_id}/detail";
    pub const PLAYER_SKILLS: &str = "/app/{space_id}/players/{player_id}/skills";
    pub const PLAYER_STAT_INCREASE: &str = "/app/{space_id}/players/{player_id}/stats/{stat}";
    pub const PLAYER_EVOLUTION_JOURNAL_WIDGET: &str =
        "/app/{space_id}/players/{player_id}/widgets/evolution-journal";
    pub const PLAYER_SPP_SPENDING_WIDGET: &str =
        "/app/{space_id}/players/{player_id}/widgets/spp-spending";

    // ── Customisation ─────────────────────────────────────────────────────────
    // Le panneau (GET) est servi par la carte 307 ; les sept mutations qui
    // suivent sont **déclarées ici et branchées par la carte 308**. Les URLs
    // vivent dès maintenant parce que le panneau les rend dans ses boutons.
    pub const PLAYER_CUSTOMISATION_WIDGET: &str =
        "/app/{space_id}/players/{player_id}/widgets/customisation";
    pub const PLAYER_CUSTOMISATION_SKILL_ADD: &str =
        "/app/{space_id}/players/{player_id}/customisation/skills/add";
    pub const PLAYER_CUSTOMISATION_STAT_ADD: &str =
        "/app/{space_id}/players/{player_id}/customisation/stats/add";
    pub const PLAYER_CUSTOMISATION_PRICE_ADJUST: &str =
        "/app/{space_id}/players/{player_id}/customisation/price/adjust";
    pub const PLAYER_CUSTOMISATION_SPP_ADD: &str =
        "/app/{space_id}/players/{player_id}/customisation/spp/add";
    pub const PLAYER_CUSTOMISATION_LINE_REMOVE: &str =
        "/app/{space_id}/players/{player_id}/customisation/lines/remove";
    pub const PLAYER_CUSTOMISATION_VALIDATE: &str =
        "/app/{space_id}/players/{player_id}/customisation/validate";
    pub const PLAYER_CUSTOMISATION_CANCEL: &str =
        "/app/{space_id}/players/{player_id}/customisation/cancel";
    /// `applied` et non `lines` : `LINE_REMOVE` retire une ligne du **panier**,
    /// celle-ci retire une customisation **déjà appliquée** au joueur. Deux
    /// gestes que rien ne rapproche, sinon le mot « retirer ».
    pub const PLAYER_CUSTOMISATION_APPLIED_REMOVE: &str =
        "/app/{space_id}/players/{player_id}/customisation/applied/remove";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn players_by_team_widget(&self, space_id: &str, team_id: &str) -> String {
        path::PLAYERS_BY_TEAM_WIDGET
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn update_roster(&self, space_id: &str, team_id: &str) -> String {
        path::PLAYERS_ROSTER_UPDATE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn match_player_selector(&self, space_id: &str, team_id: &str) -> String {
        path::MATCH_PLAYER_SELECTOR
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn player_debug(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_DEBUG
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    pub fn player_detail(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_DETAIL
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    pub fn purchase_skill(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_SKILLS
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    pub fn increase_stat(&self, space_id: &str, player_id: &str, stat: &str) -> String {
        path::PLAYER_STAT_INCREASE
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
            .replace("{stat}", stat)
    }

    pub fn evolution_journal_widget(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_EVOLUTION_JOURNAL_WIDGET
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    pub fn spp_spending_widget(&self, space_id: &str, player_id: &str) -> String {
        path::PLAYER_SPP_SPENDING_WIDGET
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }

    // ── Customisation ─────────────────────────────────────────────────────────
    // Huit routes de même forme : un gabarit, un espace, un joueur. Le
    // substituteur est factorisé plutôt que recopié huit fois.

    pub fn customisation_widget(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(path::PLAYER_CUSTOMISATION_WIDGET, space_id, player_id)
    }

    pub fn customisation_add_skill(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(path::PLAYER_CUSTOMISATION_SKILL_ADD, space_id, player_id)
    }

    pub fn customisation_add_stat(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(path::PLAYER_CUSTOMISATION_STAT_ADD, space_id, player_id)
    }

    pub fn customisation_adjust_price(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(path::PLAYER_CUSTOMISATION_PRICE_ADJUST, space_id, player_id)
    }

    pub fn customisation_add_spp(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(path::PLAYER_CUSTOMISATION_SPP_ADD, space_id, player_id)
    }

    pub fn customisation_remove_line(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(path::PLAYER_CUSTOMISATION_LINE_REMOVE, space_id, player_id)
    }

    pub fn customisation_validate(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(path::PLAYER_CUSTOMISATION_VALIDATE, space_id, player_id)
    }

    pub fn customisation_cancel(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(path::PLAYER_CUSTOMISATION_CANCEL, space_id, player_id)
    }

    pub fn customisation_remove_applied(&self, space_id: &str, player_id: &str) -> String {
        Self::pour_joueur(
            path::PLAYER_CUSTOMISATION_APPLIED_REMOVE,
            space_id,
            player_id,
        )
    }

    fn pour_joueur(gabarit: &str, space_id: &str, player_id: &str) -> String {
        gabarit
            .replace("{space_id}", space_id)
            .replace("{player_id}", player_id)
    }
}
