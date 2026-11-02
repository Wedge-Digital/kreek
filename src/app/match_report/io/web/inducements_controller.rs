use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::context::MatchReportContext;
use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::{
    InducementsTurn, MatchReportPreMatch,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{RosterPositionUid, TeamValue};
use crate::app::match_report::io::web::view_models::{team_initials, RecapSideVm};
use crate::app::match_report::io::web::write_conflict::conflict_response;
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::match_report::use_cases::inducements_recap_service::{
    self, InducementsRecap, RecapPorts,
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
/// tour, une redirection vers elle (R7, R9), ou — les deux ayant validé — le
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
        viewer: user.id,
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
    viewer: CoachId,
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
    let mr_id = page.mr_id.to_string();
    redirect_unless_turn_of(&pm, team_id, &page.routes, page.space_id, &mr_id)?;
    if pm.inducements_turn() == InducementsTurn::BothRecorded {
        return Ok(recap_page(page, &pm, can_cancel, ctx).await.into_response());
    }
    let tpl = build_vm(&pm, team_id, page.space_id, ctx, &page.routes).await?;
    Ok(tpl.into_response())
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
    })
}

/// Un `ReadyToPublish` reconverti n'a pas de valeurs d'équipe : on les relit.
async fn fill_team_values(pm: &mut MatchReportPreMatch, team_data: &dyn ITeamDataPort) {
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

/// `303` vers la page de l'équipe dont c'est le tour (carte 627) : l'outsider
/// attend le favori (R9), l'équipe qui a validé seule renvoie à l'autre (R7).
/// Quand les deux ont validé, aucune redirection : chaque adresse sert le
/// récapitulatif (R6, carte 628). Le domaine dit qui choisit ; le contrôleur
/// n'en fait qu'une adresse.
fn redirect_unless_turn_of(
    pm: &MatchReportPreMatch,
    team_id: &TeamId,
    routes: &AppRoutes,
    space_id: &str,
    match_report_id: &str,
) -> Result<(), Response> {
    let turn = match pm.inducements_turn() {
        InducementsTurn::Team(turn) if turn != team_id => turn,
        InducementsTurn::Team(_) | InducementsTurn::BothRecorded => return Ok(()),
    };
    let url = routes
        .match_report
        .inducements(space_id, match_report_id, &turn.to_string());
    Err(Redirect::to(&url).into_response())
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
    let recap = inducements_recap_service::build(pm, page.viewer, &ports).await;
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
    let (mr_id, team_id_vo) = match parse_path_ids(&match_report_id, &team_id) {
        Ok(ids) => ids,
        Err(r) => return r,
    };
    let (purchases, mercenary_purchases) = match resolve_purchases_from_form(&form) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let cmd = RecordInducementsCommand {
        match_report_id: mr_id,
        team_id: team_id_vo,
        purchases,
        mercenary_purchases,
        recorded_by: user.id,
    };
    match record_inducements_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        state.match_report.competition_data.as_ref(),
        state.match_report.player_data.as_ref(),
        &state.match_report.event_bus,
    )
    .await
    {
        Ok(RecordInducementsOutcome::RedirectToInducements { next_team_id }) => {
            let url = AppRoutes::default().match_report.inducements(
                &space_id,
                &match_report_id,
                &next_team_id,
            );
            Redirect::to(&url).into_response()
        }
        // Décision 4 (carte 628) : la seconde validation mène au récapitulatif,
        // que sert la page des coups de pouce elle-même (R6).
        Ok(RecordInducementsOutcome::RedirectToRecap) => {
            let url = AppRoutes::default().match_report.inducements(
                &space_id,
                &match_report_id,
                &team_id,
            );
            Redirect::to(&url).into_response()
        }
        Err(record_inducements_use_case::RecordInducementsError::NotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(record_inducements_use_case::RecordInducementsError::NotInPreMatchPhase) => {
            StatusCode::CONFLICT.into_response()
        }
        Err(record_inducements_use_case::RecordInducementsError::Conflict) => conflict_response(),
        Err(e) => match refus_d_achat(&e) {
            Some((statut, message)) => {
                tracing::warn!(
                    match_report_id = %match_report_id,
                    team_id = %team_id,
                    erreur = ?e,
                    "post_inducements refusé"
                );
                (statut, message).into_response()
            }
            None => {
                tracing::error!("post_inducements: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
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

    fn redirect_target(pm: &MatchReportPreMatch, team_id: &TeamId) -> Option<String> {
        redirect_unless_turn_of(pm, team_id, &AppRoutes::default(), "SP", "MR")
            .err()
            .map(|r| {
                assert_eq!(r.status(), StatusCode::SEE_OTHER);
                r.headers()["location"].to_str().unwrap().to_string()
            })
    }

    /// R9 : l'outsider dont le favori n'a pas validé est renvoyé chez lui.
    #[test]
    fn l_outsider_est_renvoye_vers_le_favori() {
        let pm = pre_match();
        let target = redirect_target(&pm, &pm.away_team_id).expect("une redirection");
        assert!(target.ends_with(&pm.home_team_id.to_string()), "{target}");
        assert_eq!(redirect_target(&pm, &pm.home_team_id), None);
    }

    /// R7 : le favori qui a validé est renvoyé vers l'outsider.
    #[test]
    fn le_favori_qui_a_valide_est_renvoye_vers_l_outsider() {
        let mut pm = pre_match();
        pm.home_inducements = Some(vec![]);
        let target = redirect_target(&pm, &pm.home_team_id).expect("une redirection");
        assert!(target.ends_with(&pm.away_team_id.to_string()), "{target}");
        assert_eq!(redirect_target(&pm, &pm.away_team_id), None);
    }

    /// R6 (carte 628) : les deux ont validé, chaque adresse sert le
    /// récapitulatif — plus aucune redirection.
    #[test]
    fn les_deux_ayant_valide_aucune_redirection() {
        let mut pm = pre_match();
        pm.home_inducements = Some(vec![]);
        pm.away_inducements = Some(vec![]);
        assert_eq!(redirect_target(&pm, &pm.home_team_id), None);
        assert_eq!(redirect_target(&pm, &pm.away_team_id), None);
        assert_eq!(pm.inducements_turn(), InducementsTurn::BothRecorded);
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
        let page = PageRequest {
            space_id: "SP",
            mr_id: &mr_id,
            viewer: CoachId::new(),
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
}
