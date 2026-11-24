//! L'après-match d'un camp, en « Chacun son équipe » (carte 658, épic E22) :
//! `GET/POST /{id}/step5/{side}`, gardé par `garde_camp`.
//!
//! Chaque coach saisit les gains, le bonus, la modification de fans et le
//! paragraphe de son équipe ; le domicile y ajoute le titre (R8). Celui de
//! l'autre s'affiche en lecture seule dans un widget rechargé sur le signal du
//! match — **le formulaire, lui, n'est jamais rechargé** : il perdrait la
//! saisie. Le camp vient de la route, que la garde a vérifiée.
//!
//! Un rapport qui n'est pas en « Chacun son équipe » renvoie à `/step5`.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::team_post_match::{SidePostMatch, TeamPostMatch};
use crate::app::match_report::domain::value_objects::{
    EntryMode, FanFactorMod, MatchGain, NoStallingBonus, TeamSide,
};
use crate::app::match_report::io::web::mvp_step_controller::{parse_side, side_slug};
use crate::app::match_report::io::web::pre_match_controller::SwitchSideVm;
use crate::app::match_report::io::web::recap_controller::{
    execute_validation, validation_response,
};
use crate::app::match_report::io::web::write_conflict::{
    conflict_response, domain_refusal_response,
};
use crate::app::match_report::ports::TeamInfoDto;
use crate::app::match_report::use_cases::match_report_access_service::{
    est_administrateur, AccesRapportDeps,
};
use crate::app::match_report::use_cases::record_team_post_match_use_case::{
    self, PostMatchStage, RecordTeamPostMatchCommand, RecordTeamPostMatchError,
};
use crate::app::match_report::use_cases::validate_team_side_use_case::{
    ValidateTeamSideCommand, ValidationStage,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

/// Une équipe du bandeau de score.
pub struct ScoreTeamVm {
    pub name: String,
    pub initials: String,
    pub logo_url: Option<String>,
    pub score: u8,
    pub cas: u8,
}

/// Le formulaire du camp : ce qu'il a enregistré, ou la proposition.
pub struct SideFormVm {
    pub gain: u32,
    pub no_stalling_bonus: bool,
    pub fan_mod: i8,
    pub paragraph: Option<String>,
    /// `Some` au domicile seul : le titre lui revient (R8).
    pub title: Option<Option<String>>,
}

#[derive(Template)]
#[template(path = "team-step5.html")]
pub struct TeamStep5Template {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub own_team_name: String,
    pub home: ScoreTeamVm,
    pub away: ScoreTeamVm,
    pub form: SideFormVm,
    pub no_stalling_bonus_kpo: u32,
    pub already_recorded: bool,
    pub form_action: String,
    pub back_url: String,
    pub opponent_url: String,
    /// Le récapitulatif, une fois l'après-match des deux camps fermé.
    pub recap_url: Option<String>,
    /// L'admin peut saisir l'autre camp (§ 12.6 de E22).
    pub switch: Option<SwitchSideVm>,
    /// Le camp est validé, donc figé (carte 659-A) : « Validé ✓ », et plus
    /// d'enregistrement.
    pub validated: bool,
    /// « Valider les [mon équipe] », sur l'écran d'attente : le camp a son
    /// après-match, l'autre pas encore, et il n'a pas validé (carte 659-A).
    pub validate_url: Option<String>,
}

impl IntoResponse for TeamStep5Template {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("team-step5.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// Les identifiants d'une page : espace, compte rendu.
struct PageIds<'a> {
    space_id: &'a str,
    mr_id: &'a str,
}

/// L'avant-match du rapport, et s'il est déjà prêt à publier.
struct Loaded {
    pm: MatchReportPreMatch,
    closed: bool,
}

pub async fn get_team_step5(
    auth_session: AuthSession,
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(side) = parse_side(&side) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let ids = PageIds {
        space_id: &space_id,
        mr_id: &mr_id,
    };
    let repo = state.match_report.match_report_repo.as_ref();
    let loaded = match load_team_entry(repo, &ids).await {
        Ok(loaded) => loaded,
        Err(response) => return response,
    };
    if let Some(redirect) = mvp_first(&ids, &loaded.pm, side) {
        return redirect;
    }
    let deps = AccesRapportDeps::from_state(&state);
    let (teams, may_switch) = load_facts(&deps, &user, &ids, &loaded.pm).await;
    build_page(&ids, &loaded, side, (teams, may_switch)).into_response()
}

/// Un rapport en « Chacun son équipe », en cours ou prêt à publier. Tout autre
/// renvoie à `/step5`, qui sait le traiter.
async fn load_team_entry(
    repo: &dyn IMatchReportRepository,
    ids: &PageIds<'_>,
) -> Result<Loaded, Response> {
    let found = repo.find_by_id(ids.mr_id).await;
    let loaded = match found {
        Ok(Some(MatchReportState::PreMatch(pm))) => Loaded { pm, closed: false },
        Ok(Some(MatchReportState::ReadyToPublish(rtp))) => Loaded {
            pm: rtp.into_pre_match(),
            closed: true,
        },
        Ok(_) => return Err(to_step5(ids)),
        Err(e) => {
            tracing::error!("get_team_step5 find_by_id {}: {e}", ids.mr_id);
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };
    match loaded.pm.entry_mode {
        EntryMode::TeamEntry => Ok(loaded),
        EntryMode::SoloEntry => Err(to_step5(ids)),
    }
}

fn to_step5(ids: &PageIds<'_>) -> Response {
    let url = AppRoutes::default()
        .match_report
        .step5(ids.space_id, ids.mr_id);
    Redirect::to(&url).into_response()
}

/// L'après-match d'un camp exige son MVP (R7) : la page renvoie à son étape.
fn mvp_first(ids: &PageIds<'_>, pm: &MatchReportPreMatch, side: TeamSide) -> Option<Response> {
    (!pm.has_mvp(side)).then(|| {
        let url = AppRoutes::default()
            .match_report
            .mvp(ids.space_id, ids.mr_id, side_slug(side));
        Redirect::to(&url).into_response()
    })
}

async fn load_facts(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    ids: &PageIds<'_>,
    pm: &MatchReportPreMatch,
) -> ((TeamInfoDto, TeamInfoDto), bool) {
    let team_data = deps.team_data;
    let (home_id, away_id) = (pm.home_team_id.to_string(), pm.away_team_id.to_string());
    let (home, away) = tokio::join!(
        team_data.find_team_info(&home_id),
        team_data.find_team_info(&away_id),
    );
    let competition_id = pm.competition_id.to_string();
    let may_switch = est_administrateur(deps, user, ids.space_id, &competition_id).await;
    (
        (home.unwrap_or_default(), away.unwrap_or_default()),
        may_switch,
    )
}

fn build_page(
    ids: &PageIds<'_>,
    loaded: &Loaded,
    side: TeamSide,
    ((home, away), may_switch): ((TeamInfoDto, TeamInfoDto), bool),
) -> TeamStep5Template {
    let (pm, routes) = (&loaded.pm, AppRoutes::default().match_report);
    let opponent = side.opposite();
    let names = (home.team_name.clone(), away.team_name.clone());
    let (scores, cas) = (pm.compute_score(), pm.compute_cas());
    TeamStep5Template {
        own_team_name: side_name(&names, side).to_string(),
        home: score_team(home, scores.0, cas.0),
        away: score_team(away, scores.1, cas.1),
        form: SideFormVm::from_domain(pm, side),
        no_stalling_bonus_kpo: NoStallingBonus::AMOUNT_KPO,
        already_recorded: pm.post_match_of(side).is_some(),
        form_action: routes.step5_side(ids.space_id, ids.mr_id, side_slug(side)),
        back_url: routes.mvp(ids.space_id, ids.mr_id, side_slug(side)),
        opponent_url: routes.step5_opponent(ids.space_id, ids.mr_id, side_slug(opponent)),
        recap_url: loaded.closed.then(|| routes.recap(ids.space_id, ids.mr_id)),
        switch: may_switch.then(|| SwitchSideVm {
            url: routes.step5_side(ids.space_id, ids.mr_id, side_slug(opponent)),
            team_name: side_name(&names, opponent).to_string(),
        }),
        validated: pm.validated_by.of(side).is_some(),
        validate_url: offers_validation(loaded, side)
            .then(|| routes.step5_side_validate(ids.space_id, ids.mr_id, side_slug(side))),
        app_routes: AppRoutes::default(),
        space_id: ids.space_id.to_string(),
        match_report_id: ids.mr_id.to_string(),
    }
}

/// L'écran d'attente offre la validation (carte 659-A, décision du
/// 2026-11-24) : le camp a son après-match, n'a pas validé, et l'autre n'a pas
/// encore le sien — une fois l'après-match fermé, c'est le récapitulatif.
fn offers_validation(loaded: &Loaded, side: TeamSide) -> bool {
    let pm = &loaded.pm;
    !loaded.closed && pm.post_match_of(side).is_some() && pm.validated_by.of(side).is_none()
}

fn side_name(names: &(String, String), side: TeamSide) -> &str {
    match side {
        TeamSide::Home => &names.0,
        TeamSide::Away => &names.1,
    }
}

fn score_team(info: TeamInfoDto, score: u8, cas: u8) -> ScoreTeamVm {
    ScoreTeamVm {
        initials: initials_from(&info.team_name),
        name: info.team_name,
        logo_url: info.logo_url,
        score,
        cas,
    }
}

fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

impl SideFormVm {
    /// Ce que le camp a enregistré ; à défaut, le gain que le domaine propose.
    fn from_domain(pm: &MatchReportPreMatch, side: TeamSide) -> Self {
        let title = (side == TeamSide::Home).then(|| pm.team_post_match.title.clone());
        match pm.post_match_of(side) {
            Some(p) => Self {
                gain: p.gain.into_inner(),
                no_stalling_bonus: p.no_stalling_bonus.into_inner(),
                fan_mod: p.fan_mod.into_inner(),
                paragraph: p.paragraph,
                title,
            },
            None => Self::suggested(pm, side, title),
        }
    }

    fn suggested(pm: &MatchReportPreMatch, side: TeamSide, title: Option<Option<String>>) -> Self {
        let (home, away) = pm.suggest_gains();
        Self {
            gain: if side == TeamSide::Home { home } else { away },
            no_stalling_bonus: false,
            fan_mod: 0,
            paragraph: None,
            title,
        }
    }
}

// ── POST ──────────────────────────────────────────────────────────────────────

/// L'après-match du seul camp : son camp vient de la route, jamais du
/// formulaire.
#[derive(Deserialize)]
pub struct TeamPostMatchForm {
    pub gain: u32,
    /// Une case décochée n'est pas envoyée du tout (carte 481).
    #[serde(default, deserialize_with = "checkbox")]
    pub no_stalling_bonus: bool,
    pub fan_mod: i8,
    pub summary_title: Option<String>,
    pub summary_paragraph: Option<String>,
}

fn checkbox<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    let _ = <String as serde::Deserialize>::deserialize(d)?;
    Ok(true)
}

pub async fn post_team_step5(
    auth_session: AuthSession,
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<TeamPostMatchForm>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(cmd) = build_command(&mr_id, &side, form, user.id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let ids = PageIds {
        space_id: &space_id,
        mr_id: &mr_id,
    };
    let side = cmd.entry.side;
    let (repo, bus) = (
        state.match_report.match_report_repo.as_ref(),
        &state.match_report.event_bus,
    );
    match record_team_post_match_use_case::execute(cmd, repo, bus).await {
        Ok(stage) => Redirect::to(&url_after(&ids, side, stage)).into_response(),
        Err(e) => error_response(e),
    }
}

/// Le récapitulatif une fois l'après-match fermé ; sinon la page du camp, qui
/// attend l'autre.
fn url_after(ids: &PageIds<'_>, side: TeamSide, stage: PostMatchStage) -> String {
    let routes = AppRoutes::default().match_report;
    match stage {
        PostMatchStage::ReadyToPublish => routes.recap(ids.space_id, ids.mr_id),
        PostMatchStage::AwaitingOtherSide => {
            routes.step5_side(ids.space_id, ids.mr_id, side_slug(side))
        }
    }
}

/// La validation d'un camp depuis l'écran d'attente (carte 659-A). Le camp
/// vient de la route, que `garde_camp` a vérifiée : jamais du formulaire. Le
/// use case est celui du récapitulatif.
pub async fn post_validate_side(
    auth_session: AuthSession,
    Path((space_id, mr_id, side)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(cmd) = validation_command(&mr_id, &side, user.id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let ids = PageIds {
        space_id: &space_id,
        mr_id: &mr_id,
    };
    validate_and_return(&state, &ids, cmd).await
}

/// Le use case du récapitulatif, puis le retour sur l'écran du camp.
async fn validate_and_return(
    state: &AppState,
    ids: &PageIds<'_>,
    cmd: ValidateTeamSideCommand,
) -> Response {
    let side = cmd.team_side;
    let result = execute_validation(state, cmd).await;
    let back = url_after_validation(ids, side, result.as_ref().ok().copied());
    validation_response(result, &back)
}

fn validation_command(mr_id: &str, side: &str, by: CoachId) -> Option<ValidateTeamSideCommand> {
    Some(ValidateTeamSideCommand {
        match_report_id: MatchReportId::try_new(mr_id).ok()?,
        team_side: parse_side(side)?,
        validated_by: by,
    })
}

/// Retour sur l'écran du camp, qui dit « Validé ✓ » ; le récapitulatif si la
/// validation a publié.
fn url_after_validation(
    ids: &PageIds<'_>,
    side: TeamSide,
    stage: Option<ValidationStage>,
) -> String {
    let routes = AppRoutes::default().match_report;
    match stage {
        Some(ValidationStage::Published) => routes.recap(ids.space_id, ids.mr_id),
        _ => routes.step5_side(ids.space_id, ids.mr_id, side_slug(side)),
    }
}

fn build_command(
    mr_id: &str,
    side: &str,
    form: TeamPostMatchForm,
    recorded_by: CoachId,
) -> Option<RecordTeamPostMatchCommand> {
    Some(RecordTeamPostMatchCommand {
        match_report_id: MatchReportId::try_new(mr_id).ok()?,
        entry: TeamPostMatch {
            side: parse_side(side)?,
            post_match: SidePostMatch {
                gain: MatchGain::try_new(form.gain).ok()?,
                no_stalling_bonus: NoStallingBonus::new(form.no_stalling_bonus),
                fan_mod: FanFactorMod::try_new(form.fan_mod).ok()?,
                paragraph: filled(form.summary_paragraph),
                recorded_by,
            },
            title: filled(form.summary_title),
        },
    })
}

/// Un texte vide n'est pas un texte.
fn filled(text: Option<String>) -> Option<String> {
    text.filter(|t| !t.trim().is_empty())
}

fn error_response(e: RecordTeamPostMatchError) -> Response {
    match e {
        RecordTeamPostMatchError::Conflict => conflict_response(),
        RecordTeamPostMatchError::Domain(err) => domain_refusal_response(&err),
        RecordTeamPostMatchError::NotFound => StatusCode::NOT_FOUND.into_response(),
        RecordTeamPostMatchError::NotInCompatibleState => StatusCode::CONFLICT.into_response(),
        RecordTeamPostMatchError::Internal(msg) => {
            tracing::error!("post_team_step5: {msg}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::error::DomainError;
    use crate::app::match_report::use_cases::test_doubles::{pre_match, with_mvps};

    const MR: &str = "01M45MDK4QNZWASD6QH0BYYYRS";

    fn form(title: Option<&str>, paragraph: Option<&str>) -> TeamPostMatchForm {
        TeamPostMatchForm {
            gain: 40,
            no_stalling_bonus: true,
            fan_mod: -1,
            summary_title: title.map(str::to_string),
            summary_paragraph: paragraph.map(str::to_string),
        }
    }

    #[test]
    fn the_side_comes_from_the_route() {
        let cmd = build_command(MR, "away", form(None, Some("Rats")), CoachId::new()).unwrap();
        assert_eq!(cmd.entry.side, TeamSide::Away);
        assert_eq!(cmd.entry.post_match.paragraph.as_deref(), Some("Rats"));
        assert!(cmd.entry.post_match.no_stalling_bonus.into_inner());
    }

    #[test]
    fn a_blank_text_is_no_text() {
        let cmd = build_command(MR, "home", form(Some("  "), Some("")), CoachId::new()).unwrap();
        assert_eq!(cmd.entry.title, None);
        assert_eq!(cmd.entry.post_match.paragraph, None);
    }

    #[test]
    fn an_unknown_side_or_fan_mod_is_refused() {
        assert!(build_command(MR, "both", form(None, None), CoachId::new()).is_none());
        let mut out_of_range = form(None, None);
        out_of_range.fan_mod = 3;
        assert!(build_command(MR, "home", out_of_range, CoachId::new()).is_none());
    }

    #[test]
    fn the_page_waits_for_the_other_side_then_goes_to_the_recap() {
        let ids = PageIds {
            space_id: "S",
            mr_id: "M",
        };
        let waiting = url_after(&ids, TeamSide::Away, PostMatchStage::AwaitingOtherSide);
        assert!(waiting.ends_with("/M/step5/away"), "{waiting}");
        let closed = url_after(&ids, TeamSide::Away, PostMatchStage::ReadyToPublish);
        assert!(closed.ends_with("/M/recap"), "{closed}");
    }

    /// Le titre n'est proposé qu'au domicile ; sans saisie, le gain proposé.
    #[test]
    fn the_title_is_offered_to_the_home_side_only() {
        let pm = with_mvps(pre_match());
        assert_eq!(
            SideFormVm::from_domain(&pm, TeamSide::Home).title,
            Some(None)
        );
        let away = SideFormVm::from_domain(&pm, TeamSide::Away);
        assert_eq!(away.title, None);
        assert_eq!(away.gain, pm.suggest_gains().1);
    }

    #[test]
    fn a_recorded_side_is_shown_as_recorded() {
        let entry = TeamPostMatch {
            side: TeamSide::Away,
            post_match: SidePostMatch {
                gain: MatchGain::try_new(55).unwrap(),
                no_stalling_bonus: NoStallingBonus::new(true),
                fan_mod: FanFactorMod::try_new(2).unwrap(),
                paragraph: Some("Rats".into()),
                recorded_by: CoachId::new(),
            },
            title: None,
        };
        let pm = with_mvps(pre_match()).apply_team_post_match(&entry);
        let vm = SideFormVm::from_domain(&pm, TeamSide::Away);
        assert_eq!((vm.gain, vm.fan_mod), (55, 2));
        assert!(vm.no_stalling_bonus);
        assert_eq!(vm.paragraph.as_deref(), Some("Rats"));
    }

    #[test]
    fn a_domain_refusal_is_a_422_with_its_reason() {
        let r = error_response(RecordTeamPostMatchError::Domain(
            DomainError::TitleIsHomeOnly,
        ));
        assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            error_response(RecordTeamPostMatchError::Conflict).status(),
            StatusCode::CONFLICT
        );
    }

    use crate::app::match_report::use_cases::test_doubles::{
        RecordingRepository, ScriptedRepository,
    };
    use axum::extract::FromRequest;

    const IDS: PageIds<'static> = PageIds {
        space_id: "S",
        mr_id: "M",
    };

    fn team_entry_with_mvps() -> MatchReportPreMatch {
        let mut pm = with_mvps(pre_match());
        pm.entry_mode = EntryMode::TeamEntry;
        pm
    }

    fn side_entry(side: TeamSide) -> TeamPostMatch {
        TeamPostMatch {
            side,
            post_match: SidePostMatch {
                gain: MatchGain::try_new(40).unwrap(),
                no_stalling_bonus: NoStallingBonus::new(false),
                fan_mod: FanFactorMod::try_new(0).unwrap(),
                paragraph: None,
                recorded_by: CoachId::new(),
            },
            title: None,
        }
    }

    fn location(response: &Response) -> String {
        let header = response.headers().get("location").expect("une redirection");
        header.to_str().unwrap().to_string()
    }

    /// Un rapport en « Chacun son équipe » se charge, ouvert ou fermé.
    #[tokio::test]
    async fn a_team_entry_report_loads_open_or_closed() {
        let open = RecordingRepository::holding(MatchReportState::PreMatch(team_entry_with_mvps()));
        assert!(!load_team_entry(&open, &IDS).await.ok().unwrap().closed);
        let ready = team_entry_with_mvps()
            .apply_team_post_match(&side_entry(TeamSide::Home))
            .apply_team_post_match(&side_entry(TeamSide::Away))
            .close_post_match()
            .unwrap();
        let closed = RecordingRepository::holding(MatchReportState::ReadyToPublish(ready));
        assert!(load_team_entry(&closed, &IDS).await.ok().unwrap().closed);
    }

    /// Un rapport en « Je saisis tout », ou introuvable, renvoie à `/step5`.
    #[tokio::test]
    async fn any_other_report_goes_back_to_step5() {
        let solo = RecordingRepository::holding(MatchReportState::PreMatch(with_mvps(pre_match())));
        let response = load_team_entry(&solo, &IDS).await.err().unwrap();
        assert!(
            location(&response).ends_with("/M/step5"),
            "{}",
            location(&response)
        );
        let unknown = ScriptedRepository::reading(vec![]);
        let response = load_team_entry(&unknown, &IDS).await.err().unwrap();
        assert!(location(&response).ends_with("/M/step5"));
    }

    /// R7 : sans le MVP de son camp, la page renvoie à son étape MVP.
    #[test]
    fn a_side_without_its_mvp_goes_to_its_mvp_step() {
        let response = mvp_first(&IDS, &pre_match(), TeamSide::Away).unwrap();
        assert!(
            location(&response).contains("/M/mvp/away"),
            "{}",
            location(&response)
        );
        assert!(mvp_first(&IDS, &team_entry_with_mvps(), TeamSide::Away).is_none());
    }

    fn team(name: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_name: name.to_string(),
            ..Default::default()
        }
    }

    fn page(side: TeamSide, closed: bool, may_switch: bool) -> TeamStep5Template {
        let loaded = Loaded {
            pm: team_entry_with_mvps(),
            closed,
        };
        let teams = (team("London Rats"), team("Orcland Raiders"));
        build_page(&IDS, &loaded, side, (teams, may_switch))
    }

    /// Le coach d'un camp : sa page, sans lien vers le récapitulatif tant que
    /// l'autre n'a pas fini, ni vers l'autre camp.
    #[test]
    fn a_coach_page_shows_only_its_side() {
        let page = page(TeamSide::Home, false, false);
        assert_eq!(page.own_team_name, "London Rats");
        assert!(page.form_action.ends_with("/M/step5/home"));
        assert!(page.opponent_url.contains("away"), "{}", page.opponent_url);
        assert!(page.recap_url.is_none() && page.switch.is_none());
        assert_eq!(page.home.initials, "LR");
    }

    /// L'admin passe à l'autre camp ; le récapitulatif une fois fermé.
    #[test]
    fn the_admin_switches_side_and_a_closed_report_shows_the_recap() {
        let page = page(TeamSide::Away, true, true);
        assert_eq!(page.own_team_name, "Orcland Raiders");
        assert!(page.recap_url.unwrap().ends_with("/M/recap"));
        let switch = page.switch.unwrap();
        assert_eq!(switch.team_name, "London Rats");
        assert!(switch.url.ends_with("/M/step5/home"));
    }

    #[test]
    fn the_initials_are_the_first_two_words() {
        assert_eq!(initials_from("les gros bras"), "LG");
        assert_eq!(initials_from("Orcs"), "O");
        assert_eq!(initials_from(""), "");
    }

    async fn posted(body: &'static str) -> TeamPostMatchForm {
        let request = axum::http::Request::builder()
            .method("POST")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(body))
            .unwrap();
        Form::<TeamPostMatchForm>::from_request(request, &())
            .await
            .unwrap()
            .0
    }

    /// Une case décochée n'est pas envoyée ; cochée, elle vaut vrai.
    #[tokio::test]
    async fn the_bonus_checkbox_is_read_from_its_presence() {
        assert!(!posted("gain=40&fan_mod=0").await.no_stalling_bonus);
        assert!(
            posted("gain=40&fan_mod=0&no_stalling_bonus=on")
                .await
                .no_stalling_bonus
        );
    }

    #[test]
    fn each_refusal_has_its_status() {
        let status = |e| error_response(e).status();
        assert_eq!(
            status(RecordTeamPostMatchError::NotFound),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status(RecordTeamPostMatchError::NotInCompatibleState),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status(RecordTeamPostMatchError::Internal("base".into())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    // ── La validation depuis l'écran d'attente (carte 659-A) ───────────────

    fn loaded(pm: MatchReportPreMatch, closed: bool) -> Loaded {
        Loaded { pm, closed }
    }

    fn waiting_home() -> MatchReportPreMatch {
        team_entry_with_mvps().apply_team_post_match(&side_entry(TeamSide::Home))
    }

    /// L'écran d'attente offre la validation au camp qui a son après-match,
    /// tant qu'il n'a pas validé et que l'autre n'a pas fini.
    #[test]
    fn the_waiting_screen_offers_the_validation() {
        assert!(offers_validation(
            &loaded(waiting_home(), false),
            TeamSide::Home
        ));
        assert!(!offers_validation(
            &loaded(waiting_home(), false),
            TeamSide::Away
        ));
        assert!(!offers_validation(
            &loaded(waiting_home(), true),
            TeamSide::Home
        ));
        let validated = waiting_home().apply_side_validated(TeamSide::Home, CoachId::new());
        assert!(!offers_validation(
            &loaded(validated, false),
            TeamSide::Home
        ));
    }

    /// Le bouton, puis « Validé ✓ » : la page le sait du domaine.
    #[test]
    fn the_page_shows_the_button_then_validated() {
        let teams = || (team("London Rats"), team("Orcland Raiders"));
        let page = build_page(
            &IDS,
            &loaded(waiting_home(), false),
            TeamSide::Home,
            (teams(), false),
        );
        let url = page.validate_url.unwrap();
        assert!(url.ends_with("/M/step5/home/validate"), "{url}");
        assert!(!page.validated);
        let validated = waiting_home().apply_side_validated(TeamSide::Home, CoachId::new());
        let page = build_page(
            &IDS,
            &loaded(validated, false),
            TeamSide::Home,
            (teams(), false),
        );
        assert!(page.validated && page.validate_url.is_none());
    }

    /// Le camp vient de la route ; un camp illisible est refusé.
    #[test]
    fn the_validated_side_comes_from_the_route() {
        let by = CoachId::new();
        let cmd = validation_command(MR, "away", by).unwrap();
        assert_eq!((cmd.team_side, cmd.validated_by), (TeamSide::Away, by));
        assert!(validation_command(MR, "both", by).is_none());
        assert!(validation_command("illisible", "home", by).is_none());
    }

    /// Retour sur l'écran du camp ; au récapitulatif si la validation publie.
    #[test]
    fn after_a_validation_the_page_or_the_recap() {
        let waiting = url_after_validation(
            &IDS,
            TeamSide::Away,
            Some(ValidationStage::AwaitingOtherSide),
        );
        assert!(waiting.ends_with("/M/step5/away"), "{waiting}");
        let refused = url_after_validation(&IDS, TeamSide::Home, None);
        assert!(refused.ends_with("/M/step5/home"), "{refused}");
        let published =
            url_after_validation(&IDS, TeamSide::Away, Some(ValidationStage::Published));
        assert!(published.ends_with("/M/recap"), "{published}");
    }
}
