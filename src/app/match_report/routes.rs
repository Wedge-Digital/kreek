pub mod path {
    pub const MATCH_REPORT_NEW: &str = "/app/{space_id}/match-report/new";
    pub const MATCH_REPORT_EDIT: &str = "/app/{space_id}/match-report/{match_report_id}";
    pub const MATCH_REPORT_FROM_PAIRING: &str = "/app/{space_id}/match-report/pairing/{pairing_id}";
    pub const MATCH_REPORT_STEP2: &str = "/app/{space_id}/match-report/{match_report_id}/step2";
    /// L'avant-match d'un camp, `home` ou `away`, en « Chacun son équipe »
    /// (carte 654) : gardé par `garde_camp`.
    pub const MATCH_REPORT_STEP2_SIDE: &str =
        "/app/{space_id}/match-report/{match_report_id}/step2/{side}";
    /// Le fan factor d'un camp, en lecture seule : le bloc de l'adversaire,
    /// rechargé sur le signal du match (carte 654).
    pub const MATCH_REPORT_STEP2_FAN_FACTOR: &str =
        "/app/{space_id}/match-report/{match_report_id}/step2/{side}/fan-factor";
    pub const MATCH_REPORT_INDUCEMENTS: &str =
        "/app/{space_id}/match-report/{match_report_id}/inducements/{team_id}";
    /// L'attente de l'autre équipe, en « Chacun son équipe » (carte 655) : un
    /// widget rechargé sur le signal du match, qui rouvre la page quand vient
    /// le tour de l'équipe.
    pub const MATCH_REPORT_INDUCEMENTS_WAIT: &str =
        "/app/{space_id}/match-report/{match_report_id}/inducements/{team_id}/wait";
    /// Hors de `/inducements/…` : `/inducements/cancel` chevaucherait
    /// `/inducements/{team_id}` (carte 626).
    pub const MATCH_REPORT_CANCEL_INDUCEMENTS: &str =
        "/app/{space_id}/match-report/{match_report_id}/cancel-inducements";
    /// Le match en direct : le journal d'où se saisit le match (carte 636).
    pub const MATCH_REPORT_LIVE: &str = "/app/{space_id}/match-report/{match_report_id}/live";
    /// Le score du match en direct, en widget : il se recharge sur le signal
    /// du canal temps réel (carte 646).
    pub const MATCH_REPORT_LIVE_SCORE: &str =
        "/app/{space_id}/match-report/{match_report_id}/live/score";
    pub const MATCH_REPORT_STEP3: &str = "/app/{space_id}/match-report/{match_report_id}/step3";
    pub const MATCH_REPORT_STEP4: &str = "/app/{space_id}/match-report/{match_report_id}/step4";
    pub const MATCH_REPORT_STEP5: &str = "/app/{space_id}/match-report/{match_report_id}/step5";
    /// L'étape MVP d'un camp, `home` ou `away` (carte 637).
    pub const MATCH_REPORT_MVP: &str = "/app/{space_id}/match-report/{match_report_id}/mvp/{side}";
    /// Le tirage au sort du MVP d'un camp parmi six désignés (carte 638).
    pub const MATCH_REPORT_MVP_DRAW: &str =
        "/app/{space_id}/match-report/{match_report_id}/mvp/{side}/draw";
    pub const MATCH_REPORT_STEP3_TURN_SELECTOR: &str =
        "/app/{space_id}/match-report/{match_report_id}/step3/turn-selector";
    pub const MATCH_REPORT_STEP4_TURN_SELECTOR: &str =
        "/app/{space_id}/match-report/{match_report_id}/step4/turn-selector";
    pub const MATCH_REPORT_STEP3_TEMP_PLAYERS: &str =
        "/app/{space_id}/match-report/{match_report_id}/step3/temp-players";
    pub const MATCH_REPORT_STEP4_TEMP_PLAYERS: &str =
        "/app/{space_id}/match-report/{match_report_id}/step4/temp-players";
    pub const MATCH_REPORT_STEP3_ACTION_PANEL: &str =
        "/app/{space_id}/match-report/{match_report_id}/step3/action-panel";
    pub const MATCH_REPORT_STEP4_ACTION_PANEL: &str =
        "/app/{space_id}/match-report/{match_report_id}/step4/action-panel";
    /// Le journal des deux camps, une seule route (carte 635).
    pub const MATCH_REPORT_ACTION_LOG: &str =
        "/app/{space_id}/match-report/{match_report_id}/action-log";
    pub const MATCH_REPORT_STEP3_ACTIONS: &str =
        "/app/{space_id}/match-report/{match_report_id}/step3/actions";
    pub const MATCH_REPORT_STEP4_ACTIONS: &str =
        "/app/{space_id}/match-report/{match_report_id}/step4/actions";
    pub const MATCH_REPORT_ACTION: &str =
        "/app/{space_id}/match-report/{match_report_id}/actions/{action_id}";
    pub const MATCH_REPORT_MERCENARY_SELECTOR: &str =
        "/app/{space_id}/match-report/{match_report_id}/step2/{team_id}/mercenaires";
    pub const MATCH_REPORT_RECAP: &str = "/app/{space_id}/match-report/{match_report_id}/recap";
    pub const MATCH_REPORT_RECAP_PUBLISH: &str =
        "/app/{space_id}/match-report/{match_report_id}/recap/publish";
    pub const MATCH_REPORT_RECAP_UNPUBLISH: &str =
        "/app/{space_id}/match-report/{match_report_id}/recap/unpublish";
    // ── Impression (épic E16) ─────────────────────────────────────────────
    // `print` est un segment fixe : il prime sur `{match_report_id}`. La
    // seconde équipe est dans le chemin, contrôlée par `space_scope` (carte 594).
    pub const PRINT_INDUCEMENTS: &str = "/app/{space_id}/match-report/print/inducements/{team_id}";
    pub const PRINT_INDUCEMENTS_WITH: &str =
        "/app/{space_id}/match-report/print/inducements/{team_id}/with/{other_team_id}";
    pub const PRINT_MATCH_SHEET: &str = "/app/{space_id}/match-report/print/sheet/{team_id}";
    pub const PRINT_MATCH_SHEET_WITH: &str =
        "/app/{space_id}/match-report/print/sheet/{team_id}/with/{other_team_id}";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn print_inducements(&self, space_id: &str, team_id: &str, other: Option<&str>) -> String {
        print_url(
            path::PRINT_INDUCEMENTS,
            path::PRINT_INDUCEMENTS_WITH,
            space_id,
            team_id,
            other,
        )
    }

    pub fn print_match_sheet(&self, space_id: &str, team_id: &str, other: Option<&str>) -> String {
        print_url(
            path::PRINT_MATCH_SHEET,
            path::PRINT_MATCH_SHEET_WITH,
            space_id,
            team_id,
            other,
        )
    }

    pub fn new_match_report(&self, space_id: &str) -> String {
        path::MATCH_REPORT_NEW.replace("{space_id}", space_id)
    }

    pub fn edit_match_report(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_EDIT
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn from_pairing(&self, space_id: &str, pairing_id: &str) -> String {
        path::MATCH_REPORT_FROM_PAIRING
            .replace("{space_id}", space_id)
            .replace("{pairing_id}", pairing_id)
    }

    pub fn step2(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP2
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    /// `side` vaut `home` ou `away`.
    pub fn step2_side(&self, space_id: &str, match_report_id: &str, side: &str) -> String {
        path::MATCH_REPORT_STEP2_SIDE
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{side}", side)
    }

    /// `side` vaut `home` ou `away`.
    pub fn step2_fan_factor(&self, space_id: &str, match_report_id: &str, side: &str) -> String {
        path::MATCH_REPORT_STEP2_FAN_FACTOR
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{side}", side)
    }

    pub fn inducements(&self, space_id: &str, match_report_id: &str, team_id: &str) -> String {
        path::MATCH_REPORT_INDUCEMENTS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{team_id}", team_id)
    }

    /// L'attente de l'autre équipe (carte 655).
    pub fn inducements_wait(&self, space_id: &str, match_report_id: &str, team_id: &str) -> String {
        path::MATCH_REPORT_INDUCEMENTS_WAIT
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{team_id}", team_id)
    }

    /// L'annulation des coups de pouce (carte 626), appelée du récapitulatif
    /// (carte 628).
    pub fn cancel_inducements(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_CANCEL_INDUCEMENTS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    /// Le match en direct (carte 636).
    pub fn live(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_LIVE
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    /// Le score du match en direct, en widget (carte 646).
    pub fn live_score(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_LIVE_SCORE
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step3(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP3
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step4(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP4
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step5(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP5
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    /// `side` vaut `home` ou `away`.
    pub fn mvp(&self, space_id: &str, match_report_id: &str, side: &str) -> String {
        path::MATCH_REPORT_MVP
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{side}", side)
    }

    /// `side` vaut `home` ou `away`.
    pub fn mvp_draw(&self, space_id: &str, match_report_id: &str, side: &str) -> String {
        path::MATCH_REPORT_MVP_DRAW
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{side}", side)
    }

    pub fn step3_turn_selector(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP3_TURN_SELECTOR
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step4_turn_selector(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP4_TURN_SELECTOR
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step3_temp_players(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP3_TEMP_PLAYERS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step4_temp_players(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP4_TEMP_PLAYERS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step3_action_panel(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP3_ACTION_PANEL
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step4_action_panel(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP4_ACTION_PANEL
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn action_log(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_ACTION_LOG
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step3_post_action(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP3_ACTIONS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn step4_post_action(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_STEP4_ACTIONS
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn delete_action(&self, space_id: &str, match_report_id: &str, action_id: &str) -> String {
        path::MATCH_REPORT_ACTION
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{action_id}", action_id)
    }

    pub fn mercenary_selector(
        &self,
        space_id: &str,
        match_report_id: &str,
        team_id: &str,
    ) -> String {
        path::MATCH_REPORT_MERCENARY_SELECTOR
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
            .replace("{team_id}", team_id)
    }

    pub fn recap(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_RECAP
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn recap_publish(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_RECAP_PUBLISH
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }

    pub fn recap_unpublish(&self, space_id: &str, match_report_id: &str) -> String {
        path::MATCH_REPORT_RECAP_UNPUBLISH
            .replace("{space_id}", space_id)
            .replace("{match_report_id}", match_report_id)
    }
}

/// L'URL d'un fragment d'impression, à une ou deux équipes.
fn print_url(
    single: &str,
    with_other: &str,
    space_id: &str,
    team_id: &str,
    other: Option<&str>,
) -> String {
    let base = match other {
        Some(o) => with_other.replace("{other_team_id}", o),
        None => single.to_string(),
    };
    base.replace("{space_id}", space_id)
        .replace("{team_id}", team_id)
}
