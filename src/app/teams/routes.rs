pub mod path {
    pub const TEAM_DETAIL: &str = "/app/{space_id}/teams/{team_id}";
    pub const DISMISS_TEAM: &str = "/app/{space_id}/teams/{team_id}/dismiss";
    pub const PENDING_ENROLLMENT_WIDGET: &str = "/app/{space_id}/team/widgets/pending";
    pub const ENROLLED_TEAMS_WIDGET: &str = "/app/{space_id}/team/widgets/enrolled";
    pub const MY_TEAMS_WIDGET: &str = "/app/{space_id}/team/widgets/my-teams";
    pub const APPROVE_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/approve";
    pub const REJECT_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/reject";
    pub const DISMISS_ENROLLMENT: &str = "/app/{space_id}/team/{team_id}/enrollment/dismiss";
    pub const APPROVE_ALL_ENROLLMENTS: &str = "/app/{space_id}/team/widgets/pending/approve-all";
    pub const COMPETITION_TEAMS_WIDGET: &str = "/app/team/widgets/competition-teams";
    pub const TEAM_SELECTION_WIDGET: &str = "/app/{space_id}/team/widgets/team-selection";
    pub const TEAM_SELECTION_JSON: &str = "/app/{space_id}/team/widgets/team-selection/json";
    pub const TEAM_SELECTION_TESTER: &str = "/team/widgets/tester";
    pub const TEAM_MATCH_CONTEXT_JSON: &str = "/app/{space_id}/team/widgets/match-context/json";
    pub const VALIDATE_IMPROVEMENT_PHASE: &str =
        "/app/{space_id}/teams/{team_id}/validate-improvement-phase";
    pub const VALIDATE_RECRUITMENT_PHASE: &str =
        "/app/{space_id}/teams/{team_id}/validate-recruitment-phase";
    pub const VALIDATE_DISMISSALS_PHASE: &str =
        "/app/{space_id}/teams/{team_id}/validate-dismissals-phase";

    // ── Recrutement ───────────────────────────────────────────────────────
    pub const RECRUITMENT_PAGE: &str = "/app/{space_id}/teams/{team_id}/recruitment";
    pub const RECRUITMENT_CATALOG_WIDGET: &str =
        "/app/{space_id}/teams/{team_id}/widgets/recruitment-catalog";
    pub const RECRUITMENT_CART_WIDGET: &str =
        "/app/{space_id}/teams/{team_id}/widgets/recruitment-cart";
    pub const RECRUITMENT_ADD_PLAYER: &str =
        "/app/{space_id}/teams/{team_id}/recruitment/players/add";
    pub const RECRUITMENT_REMOVE_PLAYER: &str =
        "/app/{space_id}/teams/{team_id}/recruitment/players/remove";
    pub const RECRUITMENT_ADD_STAFF: &str = "/app/{space_id}/teams/{team_id}/recruitment/staff/add";
    pub const RECRUITMENT_REMOVE_STAFF: &str =
        "/app/{space_id}/teams/{team_id}/recruitment/staff/remove";

    // ── Renvois ───────────────────────────────────────────────────────────
    // `mark` / `unmark`, jamais `add` / `remove` : sur une page de renvois,
    // `players/add` se lirait « ajouter un joueur à l'équipe », l'inverse exact
    // de son effet.
    pub const DISMISSALS_PAGE: &str = "/app/{space_id}/teams/{team_id}/dismissals";
    pub const DISMISSALS_ROSTER_WIDGET: &str =
        "/app/{space_id}/teams/{team_id}/widgets/dismissals-roster";
    pub const DISMISSALS_CART_WIDGET: &str =
        "/app/{space_id}/teams/{team_id}/widgets/dismissals-cart";
    pub const DISMISSALS_MARK_PLAYER: &str =
        "/app/{space_id}/teams/{team_id}/dismissals/players/mark";
    pub const DISMISSALS_UNMARK_PLAYER: &str =
        "/app/{space_id}/teams/{team_id}/dismissals/players/unmark";
    pub const DISMISSALS_MARK_STAFF: &str = "/app/{space_id}/teams/{team_id}/dismissals/staff/mark";
    pub const DISMISSALS_UNMARK_STAFF: &str =
        "/app/{space_id}/teams/{team_id}/dismissals/staff/unmark";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn team_detail(&self, space_id: &str, team_id: &str) -> String {
        path::TEAM_DETAIL
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn dismiss_team(&self, space_id: &str, team_id: &str) -> String {
        path::DISMISS_TEAM
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn pending_enrollment_widget(&self, space_id: &str) -> String {
        path::PENDING_ENROLLMENT_WIDGET.replace("{space_id}", space_id)
    }
    pub fn enrolled_teams_widget(&self, space_id: &str) -> String {
        path::ENROLLED_TEAMS_WIDGET.replace("{space_id}", space_id)
    }
    pub fn my_teams_widget(&self, space_id: &str) -> String {
        path::MY_TEAMS_WIDGET.replace("{space_id}", space_id)
    }
    pub fn approve_enrollment(&self, space_id: &str, team_id: &str) -> String {
        path::APPROVE_ENROLLMENT
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn reject_enrollment(&self, space_id: &str, team_id: &str) -> String {
        path::REJECT_ENROLLMENT
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    pub fn dismiss_enrollment(&self, space_id: &str, team_id: &str) -> String {
        path::DISMISS_ENROLLMENT
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }
    /// Le handler n'a pas besoin du `space_id` — il filtre sur la saison — mais
    /// la route en porte un, et `space_scope_middleware` exige qu'il décode en
    /// ULID. Le `"_"` qui tenait la place ici rendait un `400` muet.
    pub fn approve_all_enrollments(&self, space_id: &str) -> String {
        path::APPROVE_ALL_ENROLLMENTS.replace("{space_id}", space_id)
    }
    pub fn competition_teams_widget(&self) -> String {
        path::COMPETITION_TEAMS_WIDGET.to_string()
    }
    pub fn team_selection_widget(&self, space_id: &str) -> String {
        path::TEAM_SELECTION_WIDGET.replace("{space_id}", space_id)
    }
    pub fn team_selection_json(&self, space_id: &str) -> String {
        path::TEAM_SELECTION_JSON.replace("{space_id}", space_id)
    }

    pub fn team_match_context_json(&self, space_id: &str) -> String {
        path::TEAM_MATCH_CONTEXT_JSON.replace("{space_id}", space_id)
    }

    pub fn validate_improvement_phase(&self, space_id: &str, team_id: &str) -> String {
        path::VALIDATE_IMPROVEMENT_PHASE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn validate_recruitment_phase(&self, space_id: &str, team_id: &str) -> String {
        path::VALIDATE_RECRUITMENT_PHASE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    pub fn validate_dismissals_phase(&self, space_id: &str, team_id: &str) -> String {
        path::VALIDATE_DISMISSALS_PHASE
            .replace("{space_id}", space_id)
            .replace("{team_id}", team_id)
    }

    // ── Recrutement ───────────────────────────────────────────────────────

    pub fn recruitment_page(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_PAGE, space_id, team_id)
    }
    pub fn recruitment_catalog_widget(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_CATALOG_WIDGET, space_id, team_id)
    }
    pub fn recruitment_cart_widget(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_CART_WIDGET, space_id, team_id)
    }
    pub fn recruitment_add_player(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_ADD_PLAYER, space_id, team_id)
    }
    pub fn recruitment_remove_player(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_REMOVE_PLAYER, space_id, team_id)
    }
    pub fn recruitment_add_staff(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_ADD_STAFF, space_id, team_id)
    }
    pub fn dismissals_page(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_PAGE, space_id, team_id)
    }
    pub fn dismissals_roster_widget(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_ROSTER_WIDGET, space_id, team_id)
    }
    pub fn dismissals_cart_widget(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_CART_WIDGET, space_id, team_id)
    }
    pub fn dismissals_mark_player(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_MARK_PLAYER, space_id, team_id)
    }
    pub fn dismissals_unmark_player(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_UNMARK_PLAYER, space_id, team_id)
    }
    pub fn dismissals_mark_staff(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_MARK_STAFF, space_id, team_id)
    }
    pub fn dismissals_unmark_staff(&self, space_id: &str, team_id: &str) -> String {
        pour(path::DISMISSALS_UNMARK_STAFF, space_id, team_id)
    }

    pub fn recruitment_remove_staff(&self, space_id: &str, team_id: &str) -> String {
        pour(path::RECRUITMENT_REMOVE_STAFF, space_id, team_id)
    }
}

fn pour(gabarit: &str, space_id: &str, team_id: &str) -> String {
    gabarit
        .replace("{space_id}", space_id)
        .replace("{team_id}", team_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPACE: &str = "01KZ1J5JER8K1EPZ3444X2H45S";
    const TEAM: &str = "01M09RF4H8ZJ6S6QC1GM80EPCH";

    /// Les trois premières assertions décrivent les trois façons dont la
    /// génération a réellement échoué en production : un placeholder oublié
    /// (`{space_id}` survit), un placeholder substitué par un littéral
    /// (`"_"` → `/app/_/…`), et une valeur vide (`/app//…`). Les deux
    /// dernières produisaient un `400` muet dans `space_scope_middleware`.
    fn verifier(nom: &str, url: &str, attendus: &[&str]) {
        assert!(
            !url.contains('{'),
            "{nom} : placeholder non substitué — {url}"
        );
        assert!(!url.contains("//"), "{nom} : segment vide — {url}");
        assert!(url.starts_with('/'), "{nom} : chemin non absolu — {url}");
        for attendu in attendus {
            assert!(
                url.contains(attendu),
                "{nom} : « {attendu} » absent de {url}"
            );
        }
    }

    fn verifier_toutes(urls: &[(&str, String)], attendus: &[&str]) {
        for (nom, url) in urls {
            verifier(nom, url, attendus);
        }
    }

    fn routes_inscription() -> Vec<(&'static str, String)> {
        let r = Routes;
        vec![
            ("approve_enrollment", r.approve_enrollment(SPACE, TEAM)),
            ("reject_enrollment", r.reject_enrollment(SPACE, TEAM)),
            ("dismiss_enrollment", r.dismiss_enrollment(SPACE, TEAM)),
            ("dismiss_team", r.dismiss_team(SPACE, TEAM)),
            ("team_detail", r.team_detail(SPACE, TEAM)),
        ]
    }

    fn routes_recrutement() -> Vec<(&'static str, String)> {
        let r = Routes;
        vec![
            ("recruitment_page", r.recruitment_page(SPACE, TEAM)),
            ("catalog", r.recruitment_catalog_widget(SPACE, TEAM)),
            ("cart", r.recruitment_cart_widget(SPACE, TEAM)),
            ("add_player", r.recruitment_add_player(SPACE, TEAM)),
            ("remove_player", r.recruitment_remove_player(SPACE, TEAM)),
            ("add_staff", r.recruitment_add_staff(SPACE, TEAM)),
            ("remove_staff", r.recruitment_remove_staff(SPACE, TEAM)),
        ]
    }

    fn routes_renvois() -> Vec<(&'static str, String)> {
        let r = Routes;
        vec![
            ("dismissals_page", r.dismissals_page(SPACE, TEAM)),
            ("roster", r.dismissals_roster_widget(SPACE, TEAM)),
            ("cart", r.dismissals_cart_widget(SPACE, TEAM)),
            ("mark_player", r.dismissals_mark_player(SPACE, TEAM)),
            ("unmark_player", r.dismissals_unmark_player(SPACE, TEAM)),
            ("mark_staff", r.dismissals_mark_staff(SPACE, TEAM)),
            ("unmark_staff", r.dismissals_unmark_staff(SPACE, TEAM)),
        ]
    }

    fn routes_phases() -> Vec<(&'static str, String)> {
        let r = Routes;
        vec![
            ("improvement", r.validate_improvement_phase(SPACE, TEAM)),
            ("recruitment", r.validate_recruitment_phase(SPACE, TEAM)),
            ("dismissals", r.validate_dismissals_phase(SPACE, TEAM)),
        ]
    }

    fn routes_a_espace_seul() -> Vec<(&'static str, String)> {
        let r = Routes;
        vec![
            ("pending_widget", r.pending_enrollment_widget(SPACE)),
            ("enrolled_widget", r.enrolled_teams_widget(SPACE)),
            ("my_teams_widget", r.my_teams_widget(SPACE)),
            ("approve_all", r.approve_all_enrollments(SPACE)),
            ("selection_widget", r.team_selection_widget(SPACE)),
            ("selection_json", r.team_selection_json(SPACE)),
            ("match_context", r.team_match_context_json(SPACE)),
        ]
    }

    #[test]
    fn toutes_les_routes_a_deux_parametres_sont_bien_formees() {
        let attendus = [SPACE, TEAM];
        verifier_toutes(&routes_inscription(), &attendus);
        verifier_toutes(&routes_recrutement(), &attendus);
        verifier_toutes(&routes_renvois(), &attendus);
        verifier_toutes(&routes_phases(), &attendus);
    }

    #[test]
    fn toutes_les_routes_a_espace_seul_portent_le_space_id() {
        verifier_toutes(&routes_a_espace_seul(), &[SPACE]);
    }

    /// Régression directe du `400` muet de la démo. `approve_all_enrollments`
    /// ne prenait aucun paramètre et substituait `"_"` : le handler n'avait
    /// pas besoin du `space_id`, mais `space_scope_middleware` exige qu'il
    /// décode en ULID.
    #[test]
    fn approve_all_ne_substitue_plus_un_bouche_trou() {
        let url = Routes.approve_all_enrollments(SPACE);
        assert_eq!(
            url,
            format!("/app/{SPACE}/team/widgets/pending/approve-all"),
            "le space_id doit être celui de l'appelant, pas un caractère de remplissage"
        );
        assert!(!url.contains("/app/_/"), "bouche-trou « _ » réintroduit");
    }

    /// Une route sans paramètre reste sans placeholder — sinon elle serait
    /// ingénérable, et le défaut ne se verrait qu'au runtime.
    #[test]
    fn la_route_sans_parametre_n_a_pas_de_placeholder() {
        verifier(
            "competition_teams_widget",
            &Routes.competition_teams_widget(),
            &[],
        );
    }
}
