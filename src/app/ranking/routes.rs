pub mod path {
    pub const CLASSEMENT_WIDGET: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/widget";
    pub const DETAILED_STANDINGS_WIDGET: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/detailed-widget";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn classement_widget(
        &self,
        space_id: &str,
        competition_id: &str,
        season_id: &str,
    ) -> String {
        path::CLASSEMENT_WIDGET
            .replace("{space_id}", space_id)
            .replace("{competition_id}", competition_id)
            .replace("{season_id}", season_id)
    }

    pub fn detailed_standings_widget(
        &self,
        space_id: &str,
        competition_id: &str,
        season_id: &str,
    ) -> String {
        path::DETAILED_STANDINGS_WIDGET
            .replace("{space_id}", space_id)
            .replace("{competition_id}", competition_id)
            .replace("{season_id}", season_id)
    }
}
