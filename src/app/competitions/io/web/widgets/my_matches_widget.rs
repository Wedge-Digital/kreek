//! Le widget des cartes de « Mes matchs » (carte 677).
//!
//! La page hôte (`my_matches_controller`) ne fait que le charger. Lui rend une
//! carte par match à jouer, le bouton des prochains matchs, et « Nouveau
//! match » quand le hors calendrier est permis. Tout ce qui se décide l'est
//! ailleurs : la liste par `coach_matches_service` (675), le bouton par
//! `match_report` (676), l'assemblage par `my_matches_service`.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::context::CompetitionsContext;
use crate::app::competitions::domain::match_entry::MatchEntryKind;
// `MatchOutcome` est lu par la macro `match_widget` du gabarit.
use crate::app::competitions::io::web::resultats_view::{
    build_unlinked_match, MatchOutcome, MatchResultatVm,
};
use crate::app::competitions::io::web::widgets::print_next_opponent_widget::date_fr;
use crate::app::competitions::use_cases::my_matches_service::{
    self, MatchEntry, MyMatch, MyMatches, MyMatchesDeps,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Le bouton d'une carte.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchActionVm {
    pub label: &'static str,
    pub url: String,
    /// « Suivre » est un bouton en contour : on n'y saisit rien.
    pub outline: bool,
}

impl MatchActionVm {
    pub fn from_domain(entry: MatchEntry) -> Self {
        let (label, outline) = match entry.kind {
            MatchEntryKind::Start => ("Commencer", false),
            MatchEntryKind::Continue => ("Continuer", false),
            MatchEntryKind::Follow => ("Suivre", true),
        };
        Self {
            label,
            url: entry.url,
            outline,
        }
    }
}

/// Une carte de match.
pub struct MyMatchVm {
    pub is_next: bool,
    /// « Ligue de Condate · Journée 4 », ou la journée seule.
    pub context: String,
    /// « avant le 14/12/2026 », quand la journée a une date limite.
    pub due: Option<String>,
    pub block: MatchResultatVm,
    pub action: Option<MatchActionVm>,
}

impl MyMatchVm {
    pub fn from_domain(found: MyMatch) -> Self {
        let context = context_label(found.competition_name.as_deref(), &found.pairing.round_name);
        let due = due_label(found.pairing.round_date_end.as_deref());
        Self {
            is_next: found.is_next,
            context,
            due,
            block: build_unlinked_match(found.pairing),
            action: found.entry.map(MatchActionVm::from_domain),
        }
    }
}

fn context_label(competition: Option<&str>, round: &str) -> String {
    match competition {
        Some(name) => format!("{name} · {round}"),
        None => round.to_string(),
    }
}

fn due_label(date_end: Option<&str>) -> Option<String> {
    date_end.and_then(date_fr).map(|d| format!("avant le {d}"))
}

#[derive(Template)]
#[template(path = "widgets/my-matches-widget.html")]
pub struct MyMatchesWidgetTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub matches: Vec<MyMatchVm>,
    pub off_schedule_allowed: bool,
}

impl MyMatchesWidgetTemplate {
    pub fn from_domain(found: MyMatches, space_id: String) -> Self {
        Self {
            app_routes: AppRoutes::default(),
            space_id,
            matches: found
                .matches
                .into_iter()
                .map(MyMatchVm::from_domain)
                .collect(),
            off_schedule_allowed: found.off_schedule_allowed,
        }
    }
}

impl IntoResponse for MyMatchesWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("my_matches_widget render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

fn deps(ctx: &CompetitionsContext) -> MyMatchesDeps<'_> {
    MyMatchesDeps {
        team_info: ctx.team_info_port.as_ref(),
        match_days: ctx.match_day_repository.as_ref(),
        competitions: ctx.competition_repository.as_ref(),
        seasons: ctx.season_repository.as_ref(),
        match_entry: ctx.match_entry_port.as_ref(),
    }
}

/// Le coach est celui de la session, l'espace celui du chemin : rien d'autre
/// n'entre par l'URL.
pub async fn get_my_matches_widget(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(space) = SpaceId::try_new(&space_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match my_matches_service::find(&deps(&state.competitions), &user, &space).await {
        Ok(found) => MyMatchesWidgetTemplate::from_domain(found, space_id).into_response(),
        Err(e) => {
            tracing::error!(%space_id, error = %e, "mes matchs illisibles");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto;

    const SPACE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    fn pairing(status: &str, date_end: Option<&str>) -> PairingDisplayDto {
        PairingDisplayDto {
            pairing_id: "p1".into(),
            round_id: "r1".into(),
            round_name: "Journée 4".into(),
            round_position: 4,
            round_date_start: None,
            round_date_end: date_end.map(String::from),
            round_day_type: "fixed_date".into(),
            home_team_id: "A".into(),
            home_team_name: "Lames d'Éther".into(),
            home_roster_name: "Elfes".into(),
            home_coach_name: "Castor".into(),
            home_logo_url: None,
            home_initials: "LE".into(),
            away_team_id: "B".into(),
            away_team_name: "Gueules d'Ork".into(),
            away_roster_name: "Orques".into(),
            away_coach_name: "Morgane".into(),
            away_logo_url: None,
            away_initials: "GO".into(),
            match_status: status.into(),
            home_score: None,
            away_score: None,
            home_casualties: None,
            away_casualties: None,
            match_report_url: None,
        }
    }

    fn entry(kind: MatchEntryKind) -> MatchEntry {
        MatchEntry {
            kind,
            url: "/saisie".into(),
        }
    }

    fn my_match(is_next: bool, entry: Option<MatchEntry>) -> MyMatch {
        MyMatch {
            pairing: pairing("upcoming", Some("2026-12-14")),
            competition_name: Some("Ligue de Condate".into()),
            is_next,
            entry,
        }
    }

    fn render(matches: Vec<MyMatch>, off_schedule_allowed: bool) -> String {
        let found = MyMatches {
            matches,
            off_schedule_allowed,
        };
        MyMatchesWidgetTemplate::from_domain(found, SPACE.into())
            .render()
            .unwrap()
    }

    #[test]
    fn each_action_has_its_label_and_only_follow_is_outlined() {
        let start = MatchActionVm::from_domain(entry(MatchEntryKind::Start));
        let carry_on = MatchActionVm::from_domain(entry(MatchEntryKind::Continue));
        let follow = MatchActionVm::from_domain(entry(MatchEntryKind::Follow));
        assert_eq!((start.label, start.outline), ("Commencer", false));
        assert_eq!((carry_on.label, carry_on.outline), ("Continuer", false));
        assert_eq!((follow.label, follow.outline), ("Suivre", true));
        assert_eq!(follow.url, "/saisie");
    }

    #[test]
    fn the_context_names_the_competition_when_known() {
        assert_eq!(
            context_label(Some("Ligue"), "Journée 4"),
            "Ligue · Journée 4"
        );
        assert_eq!(context_label(None, "Journée 4"), "Journée 4");
    }

    #[test]
    fn the_due_date_is_shown_only_when_the_round_has_one() {
        assert_eq!(
            due_label(Some("2026-12-14")).as_deref(),
            Some("avant le 14/12/2026")
        );
        assert_eq!(due_label(None), None);
    }

    #[test]
    fn a_card_carries_its_block_without_link_and_its_button() {
        let vm = MyMatchVm::from_domain(my_match(true, Some(entry(MatchEntryKind::Start))));
        assert!(vm.is_next);
        assert_eq!(vm.context, "Ligue de Condate · Journée 4");
        assert_eq!(vm.block.report_url, None);
        assert_eq!(vm.action.unwrap().label, "Commencer");
    }

    #[test]
    fn without_match_the_widget_says_so() {
        let html = render(vec![], true);
        assert!(html.contains("Aucun match à jouer"), "{html}");
        assert!(html.contains("hx-disinherit=\"*\""), "{html}");
    }

    #[test]
    fn the_button_is_rendered_and_follow_is_outlined() {
        let html = render(
            vec![my_match(true, Some(entry(MatchEntryKind::Follow)))],
            true,
        );
        assert!(html.contains("Suivre"), "{html}");
        assert!(html.contains("btn-outline"), "{html}");
        assert!(html.contains("Prochain match"), "{html}");
    }

    #[test]
    fn a_match_that_is_not_next_has_no_button() {
        let html = render(vec![my_match(false, None)], true);
        assert!(
            !html.contains("Commencer") && !html.contains("Suivre"),
            "{html}"
        );
        assert!(!html.contains("Prochain match"), "{html}");
    }

    #[test]
    fn new_match_follows_the_off_schedule_rule() {
        let new_match = AppRoutes::default().match_report.new_match_report(SPACE);
        let allowed = render(vec![], true);
        assert!(allowed.contains("Nouveau match") && allowed.contains(&new_match));
        let forbidden = render(vec![], false);
        assert!(!forbidden.contains("Nouveau match"), "{forbidden}");
        assert!(!forbidden.contains(&new_match), "{forbidden}");
    }
}
