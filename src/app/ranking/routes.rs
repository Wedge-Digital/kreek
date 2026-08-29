pub mod path {
    pub const CLASSEMENT_WIDGET: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/widget";
    pub const DETAILED_STANDINGS_WIDGET: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/detailed-widget";

    // ── Points manuels (carte 452) ────────────────────────────────────────────
    //
    // La premiere page complete du BC : les deux routes ci-dessus sont des
    // fragments, embarques par les gabarits de `competitions`.
    pub const MANUAL_POINTS: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/manual-points";
    pub const MANUAL_POINTS_FORM: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/manual-points/form";
    pub const MANUAL_POINTS_LIST: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/manual-points/list";
    /// Les equipes inscrites, en JSON, pour le `kreek-select` du formulaire.
    ///
    /// **Servi par `ranking` et non emprunte a `competitions`** : aucun endpoint
    /// existant ne rend les equipes inscrites d'une saison, et le BC qui possede
    /// la page sert ses propres donnees -- depuis son propre port, celui-la meme
    /// dont le classement se nourrit deja.
    pub const MANUAL_POINTS_TEAMS_JSON: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/manual-points/teams.json";
    /// `{point_id}` est dans le chemin, jamais dans le corps (carte 416). Sa
    /// portee est tenue par le `AND season_id` du `DELETE`, pose en carte 450 :
    /// `space_scope` ne resout pas ce parametre, et un identifiant devine
    /// supprimerait sinon la ligne d'une autre competition.
    pub const MANUAL_POINT: &str =
        "/app/{space_id}/ranking/{competition_id}/{season_id}/manual-points/{point_id}";
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

    pub fn manual_points(&self, space_id: &str, competition_id: &str, season_id: &str) -> String {
        path::MANUAL_POINTS
            .replace("{space_id}", space_id)
            .replace("{competition_id}", competition_id)
            .replace("{season_id}", season_id)
    }
    pub fn manual_points_form(
        &self,
        space_id: &str,
        competition_id: &str,
        season_id: &str,
    ) -> String {
        path::MANUAL_POINTS_FORM
            .replace("{space_id}", space_id)
            .replace("{competition_id}", competition_id)
            .replace("{season_id}", season_id)
    }
    pub fn manual_points_list(
        &self,
        space_id: &str,
        competition_id: &str,
        season_id: &str,
    ) -> String {
        path::MANUAL_POINTS_LIST
            .replace("{space_id}", space_id)
            .replace("{competition_id}", competition_id)
            .replace("{season_id}", season_id)
    }
    pub fn manual_points_teams_json(
        &self,
        space_id: &str,
        competition_id: &str,
        season_id: &str,
    ) -> String {
        path::MANUAL_POINTS_TEAMS_JSON
            .replace("{space_id}", space_id)
            .replace("{competition_id}", competition_id)
            .replace("{season_id}", season_id)
    }

    pub fn manual_point(
        &self,
        space_id: &str,
        competition_id: &str,
        season_id: &str,
        point_id: i64,
    ) -> String {
        path::MANUAL_POINT
            .replace("{space_id}", space_id)
            .replace("{competition_id}", competition_id)
            .replace("{season_id}", season_id)
            .replace("{point_id}", &point_id.to_string())
    }
}
