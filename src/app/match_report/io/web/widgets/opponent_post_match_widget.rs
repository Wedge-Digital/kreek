//! L'après-match de l'adversaire, en « Chacun son équipe » (carte 658) :
//! `GET /{id}/step5/{side}/opponent`, derrière `garde_suivi`.
//!
//! La page de l'après-match d'un camp le charge pour l'autre camp, et le
//! recharge sur le signal du match : les gains de l'adversaire apparaissent
//! sans recharger la page, et le formulaire du coach n'est jamais remplacé.
//!
//! **L'écran d'attente bascule vers le récapitulatif** (carte 659-C, décision
//! du 2026-11-24) : publié, ou prêt à publier alors que le camp qui regarde a
//! validé, le rapport n'a plus rien à attendre ici — le widget rend un
//! `HX-Redirect` vers le récapitulatif, où les deux coachs se retrouvent en
//! direct. Un camp prêt à publier **sans** sa validation reste sur sa page :
//! il y revient depuis le récapitulatif pour corriger son après-match, et ne
//! doit pas en être chassé. Un brouillon ou un rapport annulé rend 204 : htmx
//! ne remplace rien.

use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::team_post_match::SidePostMatch;
use crate::app::match_report::domain::value_objects::{NoStallingBonus, TeamSide};
use crate::app::match_report::io::web::mvp_step_controller::parse_side;
use crate::app::match_report::ports::{ICoachDataPort, ITeamDataPort};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Un après-match enregistré : « 40 kPo + 10 kPo de bonus, fans +1, saisis
/// par Morgane ».
pub struct RecordedPostMatchVm {
    pub gain_kpo: u32,
    /// Le montant du bonus, s'il est pris : lu dans le domaine.
    pub bonus_kpo: Option<u32>,
    /// « +1 », « 0 », « -2 ».
    pub fan_mod: String,
    /// Le nom de qui a saisi ; absent si le port ne le connaît pas — la page
    /// n'invente pas d'auteur.
    pub recorded_by: Option<String>,
}

impl RecordedPostMatchVm {
    pub fn from_domain(post_match: &SidePostMatch, recorded_by: Option<String>) -> Self {
        let fan_mod = post_match.fan_mod.into_inner();
        Self {
            gain_kpo: post_match.gain.into_inner(),
            bonus_kpo: post_match
                .no_stalling_bonus
                .into_inner()
                .then_some(NoStallingBonus::AMOUNT_KPO),
            fan_mod: if fan_mod > 0 {
                format!("+{fan_mod}")
            } else {
                fan_mod.to_string()
            },
            recorded_by,
        }
    }
}

#[derive(Template)]
#[template(path = "opponent-post-match-widget.html")]
pub struct OpponentPostMatchTemplate {
    pub team_name: String,
    pub recorded: Option<RecordedPostMatchVm>,
}

impl IntoResponse for OpponentPostMatchTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("opponent-post-match-widget.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_opponent_post_match(
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(side) = parse_side(&side) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let report = match load_report(&state, &mr_id).await {
        Ok(report) => report,
        Err(status) => return status.into_response(),
    };
    // Le widget montre l'adversaire : le camp qui regarde est l'autre.
    let screen = waiting_screen(report, side.opposite());
    respond(&state, (&space_id, &mr_id), screen, side).await
}

/// La réponse du widget : l'adversaire, la bascule, ou rien.
async fn respond(
    state: &AppState,
    (space_id, mr_id): (&str, &str),
    screen: WaitingScreen,
    side: TeamSide,
) -> Response {
    match screen {
        WaitingScreen::Opponent(pm) => {
            let ctx = &state.match_report;
            let ports = (ctx.team_data.as_ref(), ctx.coach_data.as_ref());
            build(ports, &pm, side).await.into_response()
        }
        WaitingScreen::ToRecap => to_recap(space_id, mr_id),
        WaitingScreen::Nothing => StatusCode::NO_CONTENT.into_response(),
    }
}

/// Ce que devient l'écran d'attente, selon l'état du rapport (carte 659-C).
enum WaitingScreen {
    /// L'après-match se saisit encore : le widget montre l'adversaire.
    Opponent(MatchReportPreMatch),
    /// Plus rien à attendre : la page cède la place au récapitulatif.
    ToRecap,
    /// Rien à montrer : 204.
    Nothing,
}

/// Publié, le rapport se lit sur le récapitulatif. Prêt à publier, il y
/// renvoie le camp qui a validé — figé, il n'a plus rien à saisir ici —, et
/// garde celui qui ne l'a pas fait, venu corriger son après-match.
fn waiting_screen(report: MatchReportState, viewer: TeamSide) -> WaitingScreen {
    match report {
        MatchReportState::PreMatch(pm) => WaitingScreen::Opponent(pm),
        MatchReportState::ReadyToPublish(rtp) if rtp.validated_by.of(viewer).is_some() => {
            WaitingScreen::ToRecap
        }
        MatchReportState::ReadyToPublish(rtp) => WaitingScreen::Opponent(rtp.into_pre_match()),
        MatchReportState::Published(_) => WaitingScreen::ToRecap,
        MatchReportState::Draft(_) | MatchReportState::Cancelled(_) => WaitingScreen::Nothing,
    }
}

/// `HX-Redirect` : htmx quitte la page pour le récapitulatif.
fn to_recap(space_id: &str, mr_id: &str) -> Response {
    let url = AppRoutes::default().match_report.recap(space_id, mr_id);
    match Response::builder()
        .header("HX-Redirect", url)
        .body(axum::body::Body::empty())
    {
        Ok(response) => response,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn load_report(state: &AppState, mr_id: &str) -> Result<MatchReportState, StatusCode> {
    match state.match_report.match_report_repo.find_by_id(mr_id).await {
        Ok(Some(report)) => Ok(report),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("opponent post-match find_by_id {mr_id}: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Les deux ports que lit le widget : l'équipe, et le nom de qui a saisi.
type OpponentPorts<'a> = (&'a dyn ITeamDataPort, &'a dyn ICoachDataPort);

async fn build(
    (team_data, coach_data): OpponentPorts<'_>,
    pm: &MatchReportPreMatch,
    side: TeamSide,
) -> OpponentPostMatchTemplate {
    let team_id = pm.team_id_of(side).to_string();
    let recorded = pm.post_match_of(side);
    let author = recorded.as_ref().map(|p| p.recorded_by.to_string());
    let (info, recorded_by) = tokio::join!(team_data.find_team_info(&team_id), async {
        match &author {
            Some(id) => coach_data.find_coach_name(id).await,
            None => None,
        }
    });
    OpponentPostMatchTemplate {
        team_name: info.map(|i| i.team_name).unwrap_or_default(),
        recorded: recorded.map(|p| RecordedPostMatchVm::from_domain(&p, recorded_by)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{FanFactorMod, MatchGain};
    use crate::app::shared_kernel::identity::ids::CoachId;

    fn post_match(bonus: bool, fan_mod: i8) -> SidePostMatch {
        SidePostMatch {
            gain: MatchGain::try_new(40).unwrap(),
            no_stalling_bonus: NoStallingBonus::new(bonus),
            fan_mod: FanFactorMod::try_new(fan_mod).unwrap(),
            paragraph: None,
            recorded_by: CoachId::new(),
        }
    }

    fn render(recorded: Option<RecordedPostMatchVm>) -> String {
        OpponentPostMatchTemplate {
            team_name: "London Rats".into(),
            recorded,
        }
        .render()
        .unwrap()
    }

    #[test]
    fn the_vm_reads_gain_bonus_and_signed_fan_mod() {
        let vm = RecordedPostMatchVm::from_domain(&post_match(true, 1), None);
        assert_eq!(vm.gain_kpo, 40);
        assert_eq!(vm.bonus_kpo, Some(NoStallingBonus::AMOUNT_KPO));
        assert_eq!(vm.fan_mod, "+1");
        let vm = RecordedPostMatchVm::from_domain(&post_match(false, -2), None);
        assert_eq!((vm.bonus_kpo, vm.fan_mod.as_str()), (None, "-2"));
    }

    #[test]
    fn a_recorded_post_match_says_who_recorded_it() {
        let vm = RecordedPostMatchVm::from_domain(&post_match(false, 0), Some("Morgane".into()));
        let html = render(Some(vm));
        assert!(html.contains("London Rats"), "{html}");
        assert!(html.contains("40 kPo, fans 0"), "{html}");
        assert!(html.contains("saisis par Morgane"), "{html}");
        assert!(html.contains(r#"hx-disinherit="*""#), "{html}");
    }

    #[test]
    fn a_missing_post_match_is_awaited() {
        let html = render(None);
        assert!(html.contains("en attente"), "{html}");
        assert!(!html.contains("kPo"), "{html}");
    }

    use crate::app::match_report::domain::team_post_match::TeamPostMatch;
    use crate::app::match_report::use_cases::test_doubles::{pre_match, NamedCoaches, NamedTeams};

    fn recorded_away(by: CoachId) -> MatchReportPreMatch {
        let mut recorded = post_match(false, 0);
        recorded.recorded_by = by;
        pre_match().apply_team_post_match(&TeamPostMatch {
            side: TeamSide::Away,
            post_match: recorded,
            title: None,
        })
    }

    /// Le widget nomme l'équipe et qui a saisi, lus sur les ports.
    #[tokio::test]
    async fn the_widget_names_the_team_and_who_recorded() {
        let by = CoachId::new();
        let pm = recorded_away(by);
        let teams = NamedTeams::default().named(&pm.away_team_id.to_string(), "London Rats");
        let coaches = NamedCoaches::default().named(&by.to_string(), "Morgane");
        let widget = build((&teams, &coaches), &pm, TeamSide::Away).await;
        assert_eq!(widget.team_name, "London Rats");
        let recorded = widget.recorded.unwrap();
        assert_eq!(recorded.recorded_by.as_deref(), Some("Morgane"));
    }

    /// Un auteur inconnu du port n'est pas inventé ; un camp sans saisie attend.
    #[tokio::test]
    async fn an_unknown_author_is_not_invented() {
        let pm = recorded_away(CoachId::new());
        let (teams, coaches) = (NamedTeams::default(), NamedCoaches::default());
        let widget = build((&teams, &coaches), &pm, TeamSide::Away).await;
        assert_eq!(widget.recorded.unwrap().recorded_by, None);
        let home = build((&teams, &coaches), &pm, TeamSide::Home).await;
        assert!(home.recorded.is_none());
    }

    // ── L'écran d'attente bascule (carte 659-C) ───────────────────────────────

    use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
    use crate::app::match_report::domain::value_objects::NoStallingBonuses;

    fn ready(validated: Option<TeamSide>) -> MatchReportReadyToPublish {
        let mut pm = pre_match();
        if let Some(side) = validated {
            pm.validated_by = pm.validated_by.with(side, CoachId::new());
        }
        let gain = MatchGain::try_new(40).unwrap();
        let fan = FanFactorMod::try_new(0).unwrap();
        let bonuses = NoStallingBonuses::default();
        MatchReportReadyToPublish::from_pre_match(&pm, gain, gain, bonuses, fan, fan, None, None)
    }

    fn screen(report: MatchReportState, viewer: TeamSide) -> &'static str {
        match waiting_screen(report, viewer) {
            WaitingScreen::Opponent(_) => "opponent",
            WaitingScreen::ToRecap => "recap",
            WaitingScreen::Nothing => "nothing",
        }
    }

    /// En avant-match, rien ne change : le widget montre l'adversaire, même
    /// au camp qui a déjà validé.
    #[test]
    fn a_pre_match_report_keeps_the_waiting_screen() {
        let mut pm = pre_match();
        pm.validated_by = pm.validated_by.with(TeamSide::Away, CoachId::new());
        assert_eq!(
            screen(MatchReportState::PreMatch(pm), TeamSide::Away),
            "opponent"
        );
    }

    /// L'après-match fermé, le camp qui a validé bascule vers le
    /// récapitulatif ; l'autre reste, il peut y corriger son après-match.
    #[test]
    fn a_ready_report_sends_the_validated_side_to_the_recap() {
        let rtp = || MatchReportState::ReadyToPublish(ready(Some(TeamSide::Away)));
        assert_eq!(screen(rtp(), TeamSide::Away), "recap");
        assert_eq!(screen(rtp(), TeamSide::Home), "opponent");
        let none = MatchReportState::ReadyToPublish(ready(None));
        assert_eq!(screen(none, TeamSide::Away), "opponent");
    }

    /// Publié, il n'y a plus rien à attendre : le récapitulatif, pour tous.
    #[test]
    fn a_published_report_sends_everyone_to_the_recap() {
        let (published, _) = ready(None).publish(CoachId::new());
        let report = || MatchReportState::Published(published.clone());
        assert_eq!(screen(report(), TeamSide::Home), "recap");
        assert_eq!(screen(report(), TeamSide::Away), "recap");
    }

    #[test]
    fn the_redirection_is_an_hx_redirect_to_the_recap() {
        let response = to_recap("space-1", "mr-1");
        assert_eq!(response.status(), StatusCode::OK);
        let target = response.headers().get("HX-Redirect").unwrap();
        assert_eq!(
            target,
            &AppRoutes::default().match_report.recap("space-1", "mr-1")
        );
    }
}
