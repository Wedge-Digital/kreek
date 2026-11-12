use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::context::MatchReportContext;
use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::{
    InducementsTurn, MatchReportPreMatch,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{EntryMode, RosterPositionUid, TeamValue};
use crate::app::match_report::io::web::pre_match_controller::SwitchSideVm;
use crate::app::match_report::io::web::view_models::{team_initials, RecapSideVm};
use crate::app::match_report::io::web::write_conflict::conflict_response;
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::match_report::use_cases::inducements_recap_service::{
    self, InducementsRecap, RecapPorts,
};
use crate::app::match_report::use_cases::match_report_access_service::{
    est_administrateur, AccesRapportDeps,
};
use crate::app::match_report::use_cases::record_inducements_use_case::RecordInducementsError;
use crate::app::match_report::use_cases::record_inducements_use_case::{
    self, InducementPurchaseCmd, MercenaryLevel, MercenaryPurchaseCmd, RecordInducementsCommand,
    RecordInducementsOutcome,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::bloodbowl::inducement_definition::InducementId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "inducements.html")]
pub struct InducementsTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
    pub order_label: String,
    pub budget: u32,
    pub inducement_selector_url: String,
    pub mercenary_selector_url: String,
    pub form_action: String,
    /// « Chacun son équipe » (carte 655) : un seul bouton, « J'ai fini mes
    /// achats », panier vide compris, et la présence déclarée.
    pub team_entry: bool,
}

impl IntoResponse for InducementsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── GET ───────────────────────────────────────────────────────────────────────

/// `GET /inducements/{team_id}` : la page de choix de l'équipe dont c'est le
/// tour, une redirection vers elle (R7, R9) — en « Chacun son équipe »,
/// l'attente de l'autre équipe (carte 655) —, ou, les deux ayant validé, le
/// récapitulatif des deux camps, quel que soit `team_id` (R6, carte 628).
pub async fn get_inducements(
    auth_session: AuthSession,
    Path((space_id, match_report_id, team_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let (mr_id, team_id) = match parse_path_ids(&match_report_id, &team_id) {
        Ok(ids) => ids,
        Err(r) => return r,
    };
    let page = PageRequest {
        space_id: &space_id,
        mr_id: &mr_id,
        user: &user,
        routes: AppRoutes::default(),
    };
    match render_page(&page, &team_id, &state.match_report).await {
        Ok(response) | Err(response) => response,
    }
}

/// Ce que toute variante de la page a besoin de savoir de la requête.
struct PageRequest<'a> {
    space_id: &'a str,
    mr_id: &'a MatchReportId,
    user: &'a User,
    routes: AppRoutes,
}

async fn render_page(
    page: &PageRequest<'_>,
    team_id: &TeamId,
    ctx: &MatchReportContext,
) -> Result<Response, Response> {
    let state = load_state(ctx.match_report_repo.as_ref(), &page.mr_id.to_string()).await?;
    let can_cancel = state.can_cancel_inducements();
    let mut pm = into_pre_match(state)?;
    fill_team_values(&mut pm, ctx.team_data.as_ref()).await;
    Ok(match view_for(&pm, team_id) {
        InducementsView::RedirectTo(turn) => redirect_to_team(page, turn),
        InducementsView::Recap => recap_page(page, &pm, can_cancel, ctx).await.into_response(),
        InducementsView::Wait(waited) => wait_page(page, &pm, team_id, waited, &access_deps(ctx))
            .await
            .into_response(),
        InducementsView::Purchase => build_vm(&pm, team_id, page.space_id, ctx, &page.routes)
            .await?
            .into_response(),
    })
}

/// Ce que montre la page d'une équipe (cartes 627, 628 et 655). Le domaine dit
/// à qui c'est le tour ; le contrôleur n'en fait qu'un écran.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InducementsView<'a> {
    /// C'est le tour de l'équipe : son formulaire d'achat.
    Purchase,
    /// Les deux ont validé : le récapitulatif (R6).
    Recap,
    /// « Chacun son équipe » : l'équipe attend l'autre, sur sa propre page —
    /// l'outsider le favori (R5 de E22), le favori qui a fini l'outsider.
    Wait(&'a TeamId),
    /// « Je saisis tout » : la page de l'équipe dont c'est le tour (R7, R9).
    RedirectTo(&'a TeamId),
}

pub(crate) fn view_for<'a>(pm: &'a MatchReportPreMatch, team_id: &TeamId) -> InducementsView<'a> {
    match pm.inducements_turn() {
        InducementsTurn::BothRecorded => InducementsView::Recap,
        InducementsTurn::Team(turn) if turn == team_id => InducementsView::Purchase,
        InducementsTurn::Team(turn) if pm.entry_mode == EntryMode::TeamEntry => {
            InducementsView::Wait(turn)
        }
        InducementsTurn::Team(turn) => InducementsView::RedirectTo(turn),
    }
}

/// `303` vers la page de l'équipe dont c'est le tour (carte 627) : l'outsider
/// attend le favori (R9), l'équipe qui a validé seule renvoie à l'autre (R7).
fn redirect_to_team(page: &PageRequest<'_>, turn: &TeamId) -> Response {
    let url = page.routes.match_report.inducements(
        page.space_id,
        &page.mr_id.to_string(),
        &turn.to_string(),
    );
    Redirect::to(&url).into_response()
}

async fn build_vm(
    pm: &MatchReportPreMatch,
    team_id: &TeamId,
    space_id: &str,
    ctx: &MatchReportContext,
    routes: &AppRoutes,
) -> Result<InducementsTemplate, Response> {
    let mr_id = pm.id.clone();
    let team_data = ctx.team_data.as_ref();
    let team_info = team_data
        .find_team_info(&team_id.to_string())
        .await
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    let tier = ctx
        .competition_data
        .find_tier_rules_for_roster(&pm.season_id.to_string(), &team_info.roster_id)
        .await
        .unwrap_or_default();
    let treasury = team_data
        .find_team_treasury(&team_id.to_string())
        .await
        .unwrap_or(0);
    let budget = pm.inducement_budget_for(team_id, treasury);
    let is_topdog = pm.topdog_team_id() == team_id;
    let selector_url = build_selector_url(routes, &tier, &team_info.roster_id);
    let mercenary_url =
        routes
            .match_report
            .mercenary_selector(space_id, &mr_id.to_string(), &team_id.to_string());
    let form_action =
        routes
            .match_report
            .inducements(space_id, &mr_id.to_string(), &team_id.to_string());
    let initials = team_initials(&team_info.team_name);
    Ok(InducementsTemplate {
        app_routes: *routes,
        space_id: space_id.to_string(),
        match_report_id: mr_id.to_string(),
        team_id: team_id.to_string(),
        team_name: team_info.team_name,
        team_initials: initials,
        order_label: if is_topdog {
            "TopDog — achète en premier".to_string()
        } else {
            "Underdog — achète en second".to_string()
        },
        budget,
        inducement_selector_url: selector_url,
        mercenary_selector_url: mercenary_url,
        form_action,
        team_entry: pm.entry_mode == EntryMode::TeamEntry,
    })
}

/// Un `ReadyToPublish` reconverti n'a pas de valeurs d'équipe : on les relit.
pub(crate) async fn fill_team_values(pm: &mut MatchReportPreMatch, team_data: &dyn ITeamDataPort) {
    if pm.home_team_value.is_some() && pm.away_team_value.is_some() {
        return;
    }
    let home_id = pm.home_team_id.to_string();
    let away_id = pm.away_team_id.to_string();
    let (home_raw, away_raw) = tokio::join!(
        team_data.find_team_value(&home_id),
        team_data.find_team_value(&away_id),
    );
    pm.home_team_value = home_raw.and_then(|v| TeamValue::try_new(v).ok());
    pm.away_team_value = away_raw.and_then(|v| TeamValue::try_new(v).ok());
}

async fn load_state(
    repo: &dyn crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportState, Response> {
    repo.find_by_id(mr_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())
}

fn into_pre_match(state: MatchReportState) -> Result<MatchReportPreMatch, Response> {
    match state {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
        _ => Err(StatusCode::CONFLICT.into_response()),
    }
}

// ── Attente de l'autre équipe (carte 655) ─────────────────────────────────────

/// La page d'une équipe qui attend l'autre, en « Chacun son équipe ». Son
/// contenu est un widget, rechargé sur le signal du match, qui rouvre la page
/// quand vient le tour de l'équipe.
#[derive(Template)]
#[template(path = "inducements-wait.html")]
pub struct InducementsWaitTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub wait_url: String,
    pub pre_match_url: String,
    /// L'admin écrit les deux camps (§ 12.6 de E22) : il peut passer à
    /// l'équipe attendue.
    pub switch: Option<SwitchSideVm>,
}

impl IntoResponse for InducementsWaitTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("inducements-wait.html: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

fn access_deps(ctx: &MatchReportContext) -> AccesRapportDeps<'_> {
    AccesRapportDeps {
        admin_access: ctx.admin_access.as_ref(),
        team_data: ctx.team_data.as_ref(),
    }
}

async fn wait_page(
    page: &PageRequest<'_>,
    pm: &MatchReportPreMatch,
    team_id: &TeamId,
    waited: &TeamId,
    deps: &AccesRapportDeps<'_>,
) -> InducementsWaitTemplate {
    let mr_id = page.mr_id.to_string();
    let routes = &page.routes.match_report;
    InducementsWaitTemplate {
        app_routes: page.routes,
        space_id: page.space_id.to_string(),
        match_report_id: mr_id.clone(),
        wait_url: routes.inducements_wait(page.space_id, &mr_id, &team_id.to_string()),
        pre_match_url: routes.step2(page.space_id, &mr_id),
        switch: admin_switch(page, pm, waited, deps).await,
    }
}

/// Le lien vers la page de l'équipe attendue, pour l'admin seul.
async fn admin_switch(
    page: &PageRequest<'_>,
    pm: &MatchReportPreMatch,
    waited: &TeamId,
    deps: &AccesRapportDeps<'_>,
) -> Option<SwitchSideVm> {
    let competition_id = pm.competition_id.to_string();
    if !est_administrateur(deps, page.user, page.space_id, &competition_id).await {
        return None;
    }
    let waited = waited.to_string();
    let info = deps.team_data.find_team_info(&waited).await?;
    let mr_id = page.mr_id.to_string();
    Some(SwitchSideVm {
        url: page
            .routes
            .match_report
            .inducements(page.space_id, &mr_id, &waited),
        team_name: info.team_name,
    })
}

// ── Récapitulatif (carte 628) ─────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "inducements-recap.html")]
pub struct InducementsRecapTemplate {
    pub app_routes: AppRoutes,
    /// Le favori d'abord.
    pub sides: Vec<RecapSideVm>,
    pub topdog_name: String,
    pub underdog_name: String,
    pub kickoff_url: String,
    /// R3, posé par le domaine : sans lui, ni bouton ni confirmation.
    pub can_cancel: bool,
    pub cancel_url: String,
}

impl IntoResponse for InducementsRecapTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

async fn recap_page(
    page: &PageRequest<'_>,
    pm: &MatchReportPreMatch,
    can_cancel: bool,
    ctx: &MatchReportContext,
) -> InducementsRecapTemplate {
    let ports = RecapPorts {
        team_data: ctx.team_data.as_ref(),
        coach_data: ctx.coach_data.as_ref(),
        catalog: ctx.reference_catalog.as_ref(),
    };
    let recap = inducements_recap_service::build(pm, page.user.id, &ports).await;
    recap_template(page, recap, can_cancel)
}

fn recap_template(
    page: &PageRequest<'_>,
    recap: InducementsRecap,
    can_cancel: bool,
) -> InducementsRecapTemplate {
    let sides: Vec<RecapSideVm> = recap.sides.iter().map(RecapSideVm::from_recap).collect();
    let name_of = |i: usize| {
        sides
            .get(i)
            .map(|s| s.team_name.clone())
            .unwrap_or_default()
    };
    let (mr_id, routes) = (page.mr_id.to_string(), &page.routes.match_report);
    InducementsRecapTemplate {
        app_routes: page.routes,
        topdog_name: name_of(0),
        underdog_name: name_of(1),
        kickoff_url: routes.live(page.space_id, &mr_id),
        can_cancel,
        cancel_url: routes.cancel_inducements(page.space_id, &mr_id),
        sides,
    }
}

fn build_selector_url(
    routes: &AppRoutes,
    tier: &crate::app::match_report::ports::TierRulesDto,
    roster_id: &str,
) -> String {
    let inducement_csv: String = tier
        .allowed_inducements
        .iter()
        .map(|s| s.uid.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let star_csv: String = tier
        .allowed_star_players
        .iter()
        .map(|s| s.uid.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}?allowed_inducement_uids={}&allowed_star_player_uids={}&roster_id={}",
        routes.references.inducement_selector(),
        inducement_csv,
        star_csv,
        roster_id,
    )
}

// ── POST ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct InducementsForm {
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub selection: String,
    #[serde(default)]
    pub mercenaries: String,
}

fn resolve_purchases_from_form(
    form: &InducementsForm,
) -> Result<(Vec<InducementPurchaseCmd>, Vec<MercenaryPurchaseCmd>), Response> {
    if form.intent == "pass" {
        return Ok((Vec::new(), Vec::new()));
    }
    let purchases = parse_purchases(&form.selection);
    let mercenary_purchases = parse_mercenaries(&form.mercenaries)?;
    Ok((purchases, mercenary_purchases))
}

#[derive(Deserialize)]
struct SelectionItem {
    uid: String,
    qty: u8,
}

pub async fn post_inducements(
    auth_session: AuthSession,
    Path((space_id, match_report_id, team_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<InducementsForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let cmd = match build_command(&match_report_id, &team_id, &form, user.id) {
        Ok(cmd) => cmd,
        Err(r) => return r,
    };
    let ids = PostIds {
        space_id: &space_id,
        match_report_id: &match_report_id,
        team_id: &team_id,
    };
    match execute_command(cmd, &state).await {
        Ok(outcome) => outcome_response(&ids, outcome),
        Err(e) => record_error_response(&ids, e),
    }
}

/// Les identifiants du chemin d'un `POST`, tels que la réponse les reprend.
struct PostIds<'a> {
    space_id: &'a str,
    match_report_id: &'a str,
    team_id: &'a str,
}

fn build_command(
    match_report_id: &str,
    team_id: &str,
    form: &InducementsForm,
    recorded_by: CoachId,
) -> Result<RecordInducementsCommand, Response> {
    let (mr_id, team_id_vo) = parse_path_ids(match_report_id, team_id)?;
    let (purchases, mercenary_purchases) = resolve_purchases_from_form(form)?;
    Ok(RecordInducementsCommand {
        match_report_id: mr_id,
        team_id: team_id_vo,
        purchases,
        mercenary_purchases,
        recorded_by,
    })
}

async fn execute_command(
    cmd: RecordInducementsCommand,
    state: &AppState,
) -> Result<RecordInducementsOutcome, RecordInducementsError> {
    record_inducements_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        state.match_report.competition_data.as_ref(),
        state.match_report.player_data.as_ref(),
        &state.match_report.event_bus,
    )
    .await
}

/// Où mène une validation : la page de l'outsider en « Je saisis tout », la
/// page de l'équipe elle-même sinon — récapitulatif (R6), ou attente de
/// l'outsider en « Chacun son équipe » (carte 655).
fn outcome_response(ids: &PostIds<'_>, outcome: RecordInducementsOutcome) -> Response {
    let next_team_id = match &outcome {
        RecordInducementsOutcome::RedirectToInducements { next_team_id } => next_team_id.as_str(),
        // Décision 4 (carte 628) : la seconde validation mène au récapitulatif,
        // que sert la page des coups de pouce elle-même (R6).
        RecordInducementsOutcome::RedirectToRecap | RecordInducementsOutcome::AwaitOpponent => {
            ids.team_id
        }
    };
    let url = AppRoutes::default().match_report.inducements(
        ids.space_id,
        ids.match_report_id,
        next_team_id,
    );
    Redirect::to(&url).into_response()
}

fn record_error_response(ids: &PostIds<'_>, e: RecordInducementsError) -> Response {
    match e {
        RecordInducementsError::NotFound => StatusCode::NOT_FOUND.into_response(),
        RecordInducementsError::NotInPreMatchPhase => StatusCode::CONFLICT.into_response(),
        RecordInducementsError::Conflict => conflict_response(),
        e => refusal_response(ids, e),
    }
}

fn refusal_response(ids: &PostIds<'_>, e: RecordInducementsError) -> Response {
    match refus_d_achat(&e) {
        Some((statut, message)) => {
            tracing::warn!(
                match_report_id = %ids.match_report_id,
                team_id = %ids.team_id,
                erreur = ?e,
                "post_inducements refusé"
            );
            (statut, message).into_response()
        }
        None => {
            tracing::error!("post_inducements: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Le statut et le message d'un achat refusé — `None` pour ce qui est une
/// vraie panne.
///
/// **422 et non 500** : ces refus sont des saisies invalides, et le `Err(e)`
/// générique en faisait des 500 muets.
///
/// `UnknownInducement` signalait jusqu'ici *rien du tout* — l'achat était
/// filtré en silence après avoir été facturé (carte 406). Il vaut mieux un
/// refus lisible qu'un mercenaire évaporé.
///
/// `UnauthorizedInducement` tombait dans le générique, faute d'être atteignable
/// autrement qu'en forgeant la requête. La carte 561, en retirant de la liste
/// autorisée les coups de pouce auxquels l'équipe n'a pas droit, en a fait un
/// chemin normal — et la CI a répondu 500.
///
/// `AlreadyRecorded` et `TopdogFirst` (carte 627) sont des **409** : la saisie
/// n'est pas invalide, c'est l'état du compte rendu qui l'interdit. Leur
/// message est celui du domaine.
///
/// Les variantes restantes sont nommées une par une : une panne de lecture ou
/// d'écriture est bien un 500, et un joker aurait laissé le prochain refus y
/// retomber en silence.
fn refus_d_achat(erreur: &RecordInducementsError) -> Option<(StatusCode, String)> {
    match erreur {
        RecordInducementsError::AlreadyRecorded => Some((
            StatusCode::CONFLICT,
            DomainError::InducementsAlreadyRecorded.to_string(),
        )),
        RecordInducementsError::TopdogFirst => Some((
            StatusCode::CONFLICT,
            DomainError::TopdogHasNotChosen.to_string(),
        )),
        RecordInducementsError::Domain(e) => {
            Some((StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))
        }
        RecordInducementsError::UnauthorizedInducement(uid) => Some((
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("Coup de pouce non autorisé pour cette équipe : {uid}"),
        )),
        RecordInducementsError::NotFound
        | RecordInducementsError::NotInPreMatchPhase
        | RecordInducementsError::TeamValuesNotRecorded
        | RecordInducementsError::TreasuryUnavailable(_)
        | RecordInducementsError::TierRulesUnavailable(_)
        | RecordInducementsError::Repository(_)
        | RecordInducementsError::Conflict
        | RecordInducementsError::InvalidMercenaryPosition(_)
        | RecordInducementsError::PlayerCountUnavailable(_) => None,
    }
}

fn parse_path_ids(
    match_report_id: &str,
    team_id: &str,
) -> Result<(MatchReportId, TeamId), Response> {
    let mr_id = MatchReportId::try_new(match_report_id)
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    let team_id_vo =
        TeamId::try_new(team_id).map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    Ok((mr_id, team_id_vo))
}

#[derive(serde::Deserialize)]
struct MercenaryItem {
    position_uid: String,
    level: String,
}

fn parse_mercenaries(json: &str) -> Result<Vec<MercenaryPurchaseCmd>, Response> {
    let items: Vec<MercenaryItem> = serde_json::from_str(json).unwrap_or_default();
    items
        .into_iter()
        .filter(|m| !m.position_uid.is_empty())
        .map(|m| {
            let position_id = RosterPositionUid::try_new(&m.position_uid)
                .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
            let level = MercenaryLevel::try_from_str(&m.level)
                .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
            Ok(MercenaryPurchaseCmd { position_id, level })
        })
        .collect()
}

fn parse_purchases(selection_json: &str) -> Vec<InducementPurchaseCmd> {
    serde_json::from_str::<Vec<SelectionItem>>(selection_json)
        .unwrap_or_default()
        .into_iter()
        .filter(|item| !item.uid.is_empty())
        .map(|item| InducementPurchaseCmd {
            uid: InducementId(item.uid),
            qty: item.qty,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::use_cases::inducements_recap_service::RecapSide;
    use crate::app::match_report::use_cases::test_doubles::pre_match;

    fn statut(erreur: RecordInducementsError) -> Option<StatusCode> {
        refus_d_achat(&erreur).map(|(s, _)| s)
    }

    /// Carte 561 — le refus que la CI a reçu en 500. Le coup de pouce n'est pas
    /// dans ce que le tier autorise à cette équipe : une saisie invalide.
    #[test]
    fn un_coup_de_pouce_non_autorise_est_un_refus_pas_une_panne() {
        let erreur = RecordInducementsError::UnauthorizedInducement("WANDERING_APOTHECARY".into());
        let (statut, message) = refus_d_achat(&erreur).expect("un refus, pas une panne");

        assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            message.contains("WANDERING_APOTHECARY"),
            "le message doit nommer le coup de pouce : {message}"
        );
    }

    #[test]
    fn un_refus_du_domaine_reste_un_refus() {
        let erreur = RecordInducementsError::Domain(DomainError::UnknownInducement {
            uid: "BRIBES".into(),
        });
        assert_eq!(statut(erreur), Some(StatusCode::UNPROCESSABLE_ENTITY));
    }

    /// Le contre-exemple : une panne de lecture reste un 500. Sans lui, une
    /// fonction qui rendrait 422 pour tout passerait les deux tests ci-dessus.
    #[test]
    fn une_panne_de_lecture_reste_une_panne() {
        assert_eq!(
            statut(RecordInducementsError::TierRulesUnavailable("base".into())),
            None
        );
        assert_eq!(
            statut(RecordInducementsError::Repository("base".into())),
            None
        );
    }

    /// Carte 627 : des achats validés, ou l'outsider qui devance le favori —
    /// un état, pas une saisie invalide.
    #[test]
    fn les_refus_d_etat_rendent_409_et_disent_quoi_faire() {
        let (statut, message) =
            refus_d_achat(&RecordInducementsError::AlreadyRecorded).expect("un refus");
        assert_eq!(statut, StatusCode::CONFLICT);
        assert_eq!(
            message,
            "Les coups de pouce de cette équipe sont validés. Pour les changer, annulez ceux des deux équipes."
        );
        let (statut, message) =
            refus_d_achat(&RecordInducementsError::TopdogFirst).expect("un refus");
        assert_eq!(statut, StatusCode::CONFLICT);
        assert_eq!(message, "Le favori choisit ses coups de pouce en premier.");
    }

    /// R9 : en « Je saisis tout », l'outsider dont le favori n'a pas validé
    /// est renvoyé chez lui.
    #[test]
    fn l_outsider_est_renvoye_vers_le_favori() {
        let pm = pre_match();
        assert_eq!(
            view_for(&pm, &pm.away_team_id),
            InducementsView::RedirectTo(&pm.home_team_id)
        );
        assert_eq!(view_for(&pm, &pm.home_team_id), InducementsView::Purchase);
    }

    /// R7 : le favori qui a validé est renvoyé vers l'outsider.
    #[test]
    fn le_favori_qui_a_valide_est_renvoye_vers_l_outsider() {
        let mut pm = pre_match();
        pm.home_inducements = Some(vec![]);
        assert_eq!(
            view_for(&pm, &pm.home_team_id),
            InducementsView::RedirectTo(&pm.away_team_id)
        );
        assert_eq!(view_for(&pm, &pm.away_team_id), InducementsView::Purchase);
    }

    /// R6 (carte 628) : les deux ont validé, chaque adresse sert le
    /// récapitulatif — plus aucune redirection, dans les deux modes.
    #[test]
    fn les_deux_ayant_valide_aucune_redirection() {
        let mut pm = pre_match();
        pm.home_inducements = Some(vec![]);
        pm.away_inducements = Some(vec![]);
        assert_eq!(view_for(&pm, &pm.home_team_id), InducementsView::Recap);
        assert_eq!(view_for(&pm, &pm.away_team_id), InducementsView::Recap);
        pm.entry_mode = EntryMode::TeamEntry;
        assert_eq!(view_for(&pm, &pm.away_team_id), InducementsView::Recap);
    }

    /// Carte 655 : en « Chacun son équipe », l'outsider attend le favori sur
    /// sa propre page, puis achète quand le favori a fini.
    #[test]
    fn en_mode_b_l_outsider_attend_le_favori_chez_lui() {
        let mut pm = pre_match();
        pm.entry_mode = EntryMode::TeamEntry;
        assert_eq!(
            view_for(&pm, &pm.away_team_id),
            InducementsView::Wait(&pm.home_team_id)
        );
        assert_eq!(view_for(&pm, &pm.home_team_id), InducementsView::Purchase);
        pm.home_inducements = Some(vec![]);
        assert_eq!(view_for(&pm, &pm.away_team_id), InducementsView::Purchase);
    }

    /// Carte 655, décision 1 : le favori qui a fini attend l'outsider chez lui.
    #[test]
    fn en_mode_b_le_favori_qui_a_fini_attend_l_outsider() {
        let mut pm = pre_match();
        pm.entry_mode = EntryMode::TeamEntry;
        pm.home_inducements = Some(vec![]);
        assert_eq!(
            view_for(&pm, &pm.home_team_id),
            InducementsView::Wait(&pm.away_team_id)
        );
    }

    fn purchase_page(team_entry: bool) -> String {
        InducementsTemplate {
            app_routes: AppRoutes::default(),
            space_id: "SP".into(),
            match_report_id: "MR".into(),
            team_id: "TM".into(),
            team_name: "Nains de Fer".into(),
            team_initials: "NF".into(),
            order_label: "TopDog — achète en premier".into(),
            budget: 100,
            inducement_selector_url: "/selector".into(),
            mercenary_selector_url: "/mercenaires".into(),
            form_action: "/inducements/TM".into(),
            team_entry,
        }
        .render()
        .unwrap()
    }

    /// Carte 655 : en « Chacun son équipe », un seul bouton, « J'ai fini mes
    /// achats », et la présence déclarée ; « Je saisis tout » ne change pas.
    #[test]
    fn en_mode_b_un_seul_bouton_j_ai_fini_mes_achats() {
        let team_entry = purchase_page(true);
        assert!(
            team_entry.contains("J&#x27;ai fini mes achats")
                || team_entry.contains("J'ai fini mes achats"),
            "{team_entry}"
        );
        assert!(!team_entry.contains(r#"value="pass""#), "{team_entry}");
        assert!(
            team_entry.contains(r#"class="realtime-presence""#),
            "{team_entry}"
        );
        let solo = purchase_page(false);
        assert!(solo.contains(r#"value="pass""#), "{solo}");
        assert!(solo.contains("Valider les achats"), "{solo}");
        assert!(!solo.contains("fini mes achats"), "{solo}");
        assert!(!solo.contains(r#"class="realtime-presence""#), "{solo}");
    }

    fn wait_page_html(switch: Option<SwitchSideVm>) -> String {
        InducementsWaitTemplate {
            app_routes: AppRoutes::default(),
            space_id: "SP".into(),
            match_report_id: "MR".into(),
            wait_url: "/app/SP/match-report/MR/inducements/TM/wait".into(),
            pre_match_url: "/app/SP/match-report/MR/step2".into(),
            switch,
        }
        .render()
        .unwrap()
    }

    /// La page d'attente déclare sa présence et recharge son widget sur le
    /// signal du match ; seul l'admin peut passer à l'équipe attendue.
    #[test]
    fn la_page_d_attente_ecoute_le_match() {
        let html = wait_page_html(None);
        assert!(html.contains(r#"data-activity="write""#), "{html}");
        assert!(html.contains("/inducements/TM/wait"), "{html}");
        assert!(html.contains("matchChanged[detail.id=="), "{html}");
        assert!(!html.contains("Saisir pour"), "{html}");
        let admin = wait_page_html(Some(SwitchSideVm {
            url: "/app/SP/match-report/MR/inducements/TD".into(),
            team_name: "Nains de Fer".into(),
        }));
        assert!(admin.contains("Saisir pour Nains de Fer"), "{admin}");
    }

    fn viewer() -> User {
        use crate::app::shared_kernel::identity::coach_name::CoachName;
        use crate::app::shared_kernel::identity::email::Email;
        User::new(
            CoachId::new(),
            CoachName::try_new("Testeur".to_string()).unwrap(),
            None,
            Email::try_new("testeur@example.com".to_string()).unwrap(),
            String::new(),
        )
    }

    fn recap_side(name: &str, is_topdog: bool) -> RecapSide {
        RecapSide {
            team_id: name.into(),
            team_name: name.into(),
            is_topdog,
            lines: vec![],
            spent: 0,
            budget: 0,
            recorded_by: None,
        }
    }

    fn rendered_recap(can_cancel: bool) -> String {
        let mr_id = MatchReportId::new();
        let user = viewer();
        let page = PageRequest {
            space_id: "SP",
            mr_id: &mr_id,
            user: &user,
            routes: AppRoutes::default(),
        };
        let recap = InducementsRecap {
            sides: vec![
                recap_side("Nains de Fer", true),
                recap_side("London Rats", false),
            ],
        };
        recap_template(&page, recap, can_cancel).render().unwrap()
    }

    /// R3 : sans le feu vert du domaine, ni bouton ni confirmation ; le coup
    /// d'envoi mène toujours au match en direct (carte 636).
    #[test]
    fn l_annulation_ne_s_affiche_que_si_le_domaine_la_permet() {
        let with = rendered_recap(true);
        assert!(with.contains("Annuler les coups de pouce"));
        assert!(with.contains("/cancel-inducements"));
        assert!(with.contains("Oui, tout annuler"));
        assert!(with.contains("/live"));
        assert!(with.contains("Aucun coup de pouce"));
        let without = rendered_recap(false);
        assert!(!without.contains("Annuler les coups de pouce"));
        assert!(!without.contains("/cancel-inducements"));
        assert!(without.contains("/live"));
    }

    // ── Fonctions de la carte 655 ────────────────────────────────────────────

    use crate::app::match_report::use_cases::test_doubles::{NamedTeams, ReadyTeams};
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::identity::ids::SpaceId;

    fn page_of<'a>(user: &'a User, space_id: &'a str, mr_id: &'a MatchReportId) -> PageRequest<'a> {
        PageRequest {
            space_id,
            mr_id,
            user,
            routes: AppRoutes::default(),
        }
    }

    /// L'admin d'espace du `space_id` donné, et une équipe attendue nommée.
    struct AdminCase {
        user: User,
        space_id: String,
        mr_id: MatchReportId,
        pm: MatchReportPreMatch,
        admin: FakeAdminAccess,
        teams: NamedTeams,
    }

    fn admin_case() -> AdminCase {
        let user = viewer();
        let space = SpaceId::new();
        let pm = pre_match();
        let teams = NamedTeams::default().named(&pm.away_team_id.to_string(), "Rats");
        AdminCase {
            admin: FakeAdminAccess::new().space_admin(&user.id, &space),
            space_id: space.to_string(),
            mr_id: MatchReportId::new(),
            user,
            pm,
            teams,
        }
    }

    async fn switch_of(case: &AdminCase, admin: &FakeAdminAccess) -> Option<SwitchSideVm> {
        let page = page_of(&case.user, &case.space_id, &case.mr_id);
        let deps = AccesRapportDeps {
            admin_access: admin,
            team_data: &case.teams,
        };
        admin_switch(&page, &case.pm, &case.pm.away_team_id, &deps).await
    }

    /// Décision 2 : l'admin peut passer à l'équipe attendue.
    #[tokio::test]
    async fn l_admin_peut_saisir_pour_l_equipe_attendue() {
        let case = admin_case();
        let switch = switch_of(&case, &case.admin)
            .await
            .expect("l'admin a son lien");
        assert_eq!(switch.team_name, "Rats");
        assert_eq!(
            switch.url,
            format!(
                "/app/{}/match-report/{}/inducements/{}",
                case.space_id, case.mr_id, case.pm.away_team_id
            )
        );
    }

    #[tokio::test]
    async fn un_coach_qui_n_est_pas_admin_n_a_pas_le_lien() {
        let case = admin_case();
        assert!(switch_of(&case, &FakeAdminAccess::new()).await.is_none());
    }

    /// Une équipe que le port ne connaît pas : pas de lien sans nom.
    #[tokio::test]
    async fn une_equipe_inconnue_ne_donne_pas_de_lien() {
        let mut case = admin_case();
        case.teams = NamedTeams::default();
        assert!(switch_of(&case, &case.admin).await.is_none());
    }

    #[tokio::test]
    async fn la_page_d_attente_charge_le_widget_de_son_equipe() {
        let case = admin_case();
        let page = page_of(&case.user, &case.space_id, &case.mr_id);
        let none = FakeAdminAccess::new();
        let deps = AccesRapportDeps {
            admin_access: &none,
            team_data: &case.teams,
        };
        let home = &case.pm.home_team_id;
        let vm = wait_page(&page, &case.pm, home, &case.pm.away_team_id, &deps).await;
        let base = format!("/app/{}/match-report/{}", case.space_id, case.mr_id);
        assert_eq!(vm.wait_url, format!("{base}/inducements/{home}/wait"));
        assert_eq!(vm.pre_match_url, format!("{base}/step2"));
        assert!(vm.switch.is_none());
    }

    #[test]
    fn la_route_d_attente_porte_l_equipe() {
        let url = AppRoutes::default()
            .match_report
            .inducements_wait("SP", "MR", "TM");
        assert_eq!(url, "/app/SP/match-report/MR/inducements/TM/wait");
    }

    #[test]
    fn le_tour_d_une_autre_equipe_redirige_vers_sa_page() {
        let user = viewer();
        let mr_id = MatchReportId::new();
        let turn = TeamId::new();
        let response = redirect_to_team(&page_of(&user, "SP", &mr_id), &turn);
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers()["location"],
            format!("/app/SP/match-report/{mr_id}/inducements/{turn}").as_str()
        );
    }

    fn location_after(outcome: RecordInducementsOutcome) -> String {
        let ids = PostIds {
            space_id: "SP",
            match_report_id: "MR",
            team_id: "MINE",
        };
        let response = outcome_response(&ids, outcome);
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        response.headers()["location"].to_str().unwrap().to_string()
    }

    /// Mode A : le favori va chez l'outsider ; la seconde validation et le
    /// favori du mode B restent sur leur page (récapitulatif ou attente).
    #[test]
    fn chaque_issue_mene_a_la_bonne_page() {
        let base = "/app/SP/match-report/MR/inducements";
        let next = RecordInducementsOutcome::RedirectToInducements {
            next_team_id: "OTHER".into(),
        };
        assert_eq!(location_after(next), format!("{base}/OTHER"));
        assert_eq!(
            location_after(RecordInducementsOutcome::RedirectToRecap),
            format!("{base}/MINE")
        );
        assert_eq!(
            location_after(RecordInducementsOutcome::AwaitOpponent),
            format!("{base}/MINE")
        );
    }

    fn status_of(e: RecordInducementsError) -> StatusCode {
        let ids = PostIds {
            space_id: "SP",
            match_report_id: "MR",
            team_id: "TM",
        };
        record_error_response(&ids, e).status()
    }

    #[test]
    fn chaque_erreur_a_son_statut() {
        assert_eq!(
            status_of(RecordInducementsError::NotFound),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status_of(RecordInducementsError::NotInPreMatchPhase),
            StatusCode::CONFLICT
        );
        let refus =
            RecordInducementsError::Domain(DomainError::UnknownInducement { uid: "X".into() });
        assert_eq!(status_of(refus), StatusCode::UNPROCESSABLE_ENTITY);
        let panne = RecordInducementsError::Repository("base".into());
        assert_eq!(status_of(panne), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// La course perdue recharge la page, comme partout ailleurs (carte 653).
    #[test]
    fn un_conflit_d_ecriture_recharge_la_page() {
        let ids = PostIds {
            space_id: "SP",
            match_report_id: "MR",
            team_id: "TM",
        };
        let response = record_error_response(&ids, RecordInducementsError::Conflict);
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(response.headers().contains_key("HX-Refresh"));
    }

    fn form(intent: &str, selection: &str) -> InducementsForm {
        InducementsForm {
            intent: intent.into(),
            selection: selection.into(),
            mercenaries: "[]".into(),
        }
    }

    #[test]
    fn la_commande_reprend_le_panier_et_l_auteur() {
        let (mr_id, team) = (MatchReportId::new(), TeamId::new());
        let author = CoachId::new();
        let selection = r#"[{"uid":"BRIBES","qty":2}]"#;
        let cmd = build_command(
            &mr_id.to_string(),
            &team.to_string(),
            &form("validate", selection),
            author,
        )
        .unwrap_or_else(|_| panic!("commande valide"));
        assert_eq!(cmd.team_id, team);
        assert_eq!(cmd.recorded_by, author);
        assert_eq!(cmd.purchases.len(), 1);
        assert_eq!(cmd.purchases[0].qty, 2);
    }

    /// « Passer » (mode A) ignore le panier : une liste vide, comme « J'ai fini »
    /// panier vide en mode B.
    #[test]
    fn passer_vide_le_panier() {
        let (mr_id, team) = (MatchReportId::new(), TeamId::new());
        let selection = r#"[{"uid":"BRIBES","qty":2}]"#;
        let cmd = build_command(
            &mr_id.to_string(),
            &team.to_string(),
            &form("pass", selection),
            CoachId::new(),
        )
        .unwrap_or_else(|_| panic!("commande valide"));
        assert!(cmd.purchases.is_empty());
    }

    #[test]
    fn un_identifiant_illisible_rend_400() {
        let team = TeamId::new().to_string();
        let refused = build_command("pas-un-id", &team, &form("validate", "[]"), CoachId::new());
        match refused {
            Err(response) => assert_eq!(response.status(), StatusCode::BAD_REQUEST),
            Ok(_) => panic!("un identifiant illisible doit être refusé"),
        }
    }

    /// Les valeurs déjà portées ne sont pas relues ; celles qui manquent le sont,
    /// et une équipe que le port ignore reste sans valeur — rien n'est inventé.
    #[tokio::test]
    async fn les_valeurs_d_equipe_ne_sont_relues_que_si_elles_manquent() {
        let mut pm = pre_match();
        pm.home_team_value = Some(TeamValue::try_new(900).unwrap());
        fill_team_values(&mut pm, &ReadyTeams).await;
        assert_eq!(pm.home_team_value, Some(TeamValue::try_new(900).unwrap()));

        pm.away_team_value = None;
        fill_team_values(&mut pm, &ReadyTeams).await;
        assert_eq!(pm.away_team_value, Some(TeamValue::try_new(1000).unwrap()));

        pm.away_team_value = None;
        fill_team_values(&mut pm, &NamedTeams::default()).await;
        assert_eq!(pm.away_team_value, None);
    }
}
