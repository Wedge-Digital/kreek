use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_draft::MatchReportDraft;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{EntryMode, MatchReportOrigin};
use crate::app::match_report::io::web::write_conflict::conflict_response;
use crate::app::match_report::ports::{
    CreationAppariementError, ICompetitionDataPort, ITeamDataPort,
};
use crate::app::match_report::use_cases::match_report_access_service::may_choose_entry_mode;
use crate::app::match_report::use_cases::{
    create_match_report_use_case, update_match_selection_use_case,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "match-selection.html")]
pub struct MatchSelectionTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub widget_url: String,
    pub team_widget_url: String,
    pub is_prefilled: bool,
    pub error_message: Option<String>,
    pub form_action: String,
    /// La valeur que porte l'option « Je saisis tout » de la carte « Qui
    /// saisit ? » (carte 632) — celle que `read_entry_mode` relit.
    pub solo_entry_value: &'static str,
}

impl IntoResponse for MatchSelectionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Le démarrage d'un match du calendrier (carte 633) : le match, la carte
/// « Qui saisit ? » de la carte 632, et « Commencer → ». Pas de sélection
/// d'équipes — un match du calendrier garde les siennes (R4).
#[derive(Template)]
#[template(path = "match-mode.html")]
pub struct MatchModeTemplate {
    pub app_routes: AppRoutes,
    pub form_action: String,
    pub summary: MatchSummaryVm,
    /// Cf. `MatchSelectionTemplate::solo_entry_value`.
    pub solo_entry_value: &'static str,
}

impl IntoResponse for MatchModeTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Le match qu'on s'apprête à démarrer.
///
/// **Des noms, pas des identifiants.** Un ULID n'apprend rien au coach.
pub struct MatchSummaryVm {
    pub competition_name: String,
    pub round_name: String,
    pub home_team_name: String,
    pub away_team_name: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn build_widget_url(space_id: &str) -> String {
    AppRoutes::default()
        .competitions
        .competition_widget(space_id)
        + "?show_rounds=true"
}

fn build_widget_url_prefilled(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    round_id: &str,
) -> String {
    format!(
        "{}?show_rounds=true&competition_id={}&season_id={}&round_id={}",
        AppRoutes::default()
            .competitions
            .competition_widget(space_id),
        competition_id,
        season_id,
        round_id,
    )
}

fn build_team_widget_url(space_id: &str) -> String {
    AppRoutes::default().teams.team_selection_widget(space_id)
}

fn build_team_widget_url_prefilled(
    space_id: &str,
    season_id: &str,
    home_id: &str,
    away_id: &str,
) -> String {
    format!(
        "{}?season_id={}&selected_home={}&selected_away={}",
        AppRoutes::default().teams.team_selection_widget(space_id),
        season_id,
        home_id,
        away_id,
    )
}

// ── Handlers GET ─────────────────────────────────────────────────────────────

pub async fn from_pairing(
    Path((space_id, pairing_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mr_id = match state
        .match_report
        .match_report_repo
        .find_id_by_pairing(&pairing_id)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("from_pairing find_id_by_pairing {pairing_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let url = AppRoutes::default()
        .match_report
        .edit_match_report(&space_id, &mr_id);
    Redirect::to(&url).into_response()
}

/// Les ports que lit l'écran d'un rapport existant.
///
/// **Aucun ne confirme** (carte 633, R3) : ouvrir le lien d'un match ne
/// verrouille rien et ne le passe pas « en cours ». Le dépôt n'y sert qu'à
/// lire l'état.
pub(crate) struct EditPagePorts<'a> {
    pub repo: &'a dyn IMatchReportRepository,
    pub competition_data: &'a dyn ICompetitionDataPort,
    pub team_data: &'a dyn ITeamDataPort,
}

/// Le match d'un brouillon, en noms.
///
/// Les quatre noms sont résolus par les ports qui existent déjà —
/// `find_round_context` pour la compétition et la journée, `find_team_info` pour
/// les deux équipes. Un nom manquant devient l'identifiant : mieux qu'une case
/// vide, et le problème se voit.
async fn match_summary(ports: &EditPagePorts<'_>, draft: &MatchReportDraft) -> MatchSummaryVm {
    let (comp_id, round_id) = (draft.competition_id.to_string(), draft.round_id.to_string());
    let (home_id, away_id) = (
        draft.home_team_id.to_string(),
        draft.away_team_id.to_string(),
    );
    let season_id = draft.season_id.to_string();
    let contexte = ports
        .competition_data
        .find_round_context(&season_id, &round_id)
        .await;
    let teams = ports.team_data;
    let (home, away) = tokio::join!(
        teams.find_team_info(&home_id),
        teams.find_team_info(&away_id)
    );

    MatchSummaryVm {
        competition_name: contexte
            .as_ref()
            .map(|c| c.competition_name.clone())
            .unwrap_or(comp_id),
        round_name: contexte.map(|c| c.round_name).unwrap_or(round_id),
        home_team_name: home.map(|t| t.team_name).unwrap_or(home_id),
        away_team_name: away.map(|t| t.team_name).unwrap_or(away_id),
    }
}

pub async fn new_match_report(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let form_action = AppRoutes::default()
        .match_report
        .new_match_report(&space_id);

    MatchSelectionTemplate {
        app_routes: Default::default(),
        widget_url: build_widget_url(&space_id),
        team_widget_url: build_team_widget_url(&space_id),
        space_id,
        is_prefilled: false,
        error_message: None,
        form_action,
        solo_entry_value: EntryMode::SoloEntry.as_str(),
    }
    .into_response()
}

pub async fn edit_match_report(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if auth_session.user.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let ports = EditPagePorts {
        repo: state.match_report.match_report_repo.as_ref(),
        competition_data: state.match_report.competition_data.as_ref(),
        team_data: state.match_report.team_data.as_ref(),
    };
    edit_page(&ports, &space_id, &match_report_id).await
}

/// L'aiguillage d'un rapport existant selon son état.
///
/// Un brouillon du calendrier mène à « Qui saisit ? » (carte 633) ; un
/// brouillon hors calendrier, s'il en reste d'avant la carte 552, garde la
/// sélection.
pub(crate) async fn edit_page(
    ports: &EditPagePorts<'_>,
    space_id: &str,
    match_report_id: &str,
) -> Response {
    let mr_state = match load_state(ports.repo, match_report_id).await {
        Ok(s) => s,
        Err(refus) => return refus,
    };
    let routes = AppRoutes::default().match_report;
    match mr_state {
        MatchReportState::Draft(draft) if draft.keeps_its_teams() => {
            match_mode_page(ports, space_id, &draft).await
        }
        MatchReportState::Draft(draft) => selection_page(space_id, &draft),
        MatchReportState::PreMatch(_) => {
            Redirect::to(&routes.step2(space_id, match_report_id)).into_response()
        }
        MatchReportState::ReadyToPublish(_) => {
            Redirect::to(&routes.step5(space_id, match_report_id)).into_response()
        }
        MatchReportState::Cancelled(_) => StatusCode::GONE.into_response(),
        MatchReportState::Published(_) => StatusCode::CONFLICT.into_response(),
    }
}

async fn load_state(
    repo: &dyn IMatchReportRepository,
    match_report_id: &str,
) -> Result<MatchReportState, Response> {
    match repo.find_by_id(match_report_id).await {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("edit_match_report find_by_id {match_report_id}: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

async fn match_mode_page(
    ports: &EditPagePorts<'_>,
    space_id: &str,
    draft: &MatchReportDraft,
) -> Response {
    MatchModeTemplate {
        app_routes: Default::default(),
        form_action: AppRoutes::default()
            .match_report
            .edit_match_report(space_id, &draft.id.to_string()),
        summary: match_summary(ports, draft).await,
        solo_entry_value: EntryMode::SoloEntry.as_str(),
    }
    .into_response()
}

fn selection_page(space_id: &str, draft: &MatchReportDraft) -> Response {
    let comp_id = draft.competition_id.to_string();
    let season_id = draft.season_id.to_string();
    let round_id = draft.round_id.to_string();
    let home_id = draft.home_team_id.to_string();
    let away_id = draft.away_team_id.to_string();

    MatchSelectionTemplate {
        app_routes: Default::default(),
        widget_url: build_widget_url_prefilled(space_id, &comp_id, &season_id, &round_id),
        team_widget_url: build_team_widget_url_prefilled(space_id, &season_id, &home_id, &away_id),
        space_id: space_id.to_string(),
        is_prefilled: true,
        error_message: None,
        form_action: AppRoutes::default()
            .match_report
            .edit_match_report(space_id, &draft.id.to_string()),
        solo_entry_value: EntryMode::SoloEntry.as_str(),
    }
    .into_response()
}

// ── POST form ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateMatchReportForm {
    pub competition_id: String,
    pub season_id: String,
    pub round_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
    /// La réponse à « Qui saisit ? » (carte 632) ; cf. `read_entry_mode`.
    pub entry_mode: Option<String>,
}

/// Le corps du POST de la phase 1 sur un rapport **existant**.
///
/// Distinct de `CreateMatchReportForm`, et ses deux champs sont facultatifs
/// (carte 564) : l'écran « Qui saisit ? » d'un match du calendrier (carte 633)
/// n'a aucun champ d'équipe, et le formulaire n'envoie que le mode. Exiger les cinq champs de la création
/// faisait échouer l'extraction avant le handler — `missing field
/// competition_id` — alors même que ces valeurs auraient été ignorées. Le coach
/// ne pouvait plus commencer son rapport.
///
/// La compétition, la saison et la journée ne sont pas lues : elles ne changent
/// pas après la création. Le widget les envoie quand même ; elles sont écartées.
#[derive(Deserialize)]
pub struct UpdateMatchSelectionForm {
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
    /// La réponse à « Qui saisit ? » (carte 632) ; cf. `read_entry_mode`.
    pub entry_mode: Option<String>,
}

/// Traduit le champ `entry_mode` du formulaire en mode de saisie (carte 632).
///
/// **Absent, il vaut « Je saisis tout »**, le défaut déclaré par l'épic E21
/// (décision 2 du cadrage) : les tests et les appels qui postent sans la carte
/// « Qui saisit ? » démarrent comme avant. Une valeur inconnue est une requête
/// mal formée — `Err`, que le handler rend en 400. « Chacun son équipe »
/// (`TeamEntry`, carte 652) est lu, mais l'option reste grisée jusqu'à la
/// carte 659 : seul un POST direct l'envoie.
pub(crate) fn read_entry_mode(field: Option<&str>) -> Result<EntryMode, ()> {
    match field {
        None => Ok(EntryMode::SoloEntry),
        Some(value) => EntryMode::from_written(value).ok_or(()),
    }
}

/// Le mode du formulaire, et le droit de le choisir pour ces deux équipes :
/// « Chacun son équipe » exige deux coachs distincts (§ 12.9 de E22). Les
/// deux refus rendent `None`, que le handler rend en 400 avant tout effet.
async fn mode_choisi(
    team_data: &dyn ITeamDataPort,
    field: Option<&str>,
    home_team_id: &str,
    away_team_id: &str,
) -> Option<EntryMode> {
    let mode = read_entry_mode(field).ok()?;
    if may_choose_entry_mode(team_data, home_team_id, away_team_id, mode).await {
        return Some(mode);
    }
    tracing::info!(
        home_team_id,
        away_team_id,
        "mode « Chacun son équipe » refusé pour ce match"
    );
    None
}

/// `mode_choisi` sur le formulaire de création : les équipes qu'il envoie.
async fn mode_du_formulaire(state: &AppState, form: &CreateMatchReportForm) -> Option<EntryMode> {
    let team_data = state.match_report.team_data.as_ref();
    let field = form.entry_mode.as_deref();
    mode_choisi(team_data, field, &form.home_team_id, &form.away_team_id).await
}

/// `mode_choisi` sur un rapport existant : les équipes du brouillon, lues sur
/// la projection. Un rapport introuvable ou illisible refuse.
async fn mode_du_rapport(
    state: &AppState,
    match_report_id: &str,
    field: Option<&str>,
) -> Option<EntryMode> {
    let repo = state.match_report.match_report_repo.as_ref();
    let Ok(Some((_, home, away))) = repo.find_access_scope(match_report_id).await else {
        tracing::warn!("update_match_selection : portée de {match_report_id} illisible");
        return None;
    };
    mode_choisi(state.match_report.team_data.as_ref(), field, &home, &away).await
}

// ── Handlers POST ────────────────────────────────────────────────────────────

pub async fn create_match_report(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<CreateMatchReportForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    // Lu avant tout effet : un mode inconnu ne doit rien programmer.
    let Some(entry_mode) = mode_du_formulaire(&state, &form).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if let Err(refus) = garde_hors_calendrier(&state, &form).await {
        return refus;
    }

    // Carte 552 — **l'appariement d'abord**. Le rapport naît ensuite en le
    // portant, au lieu qu'un appariement lui soit fabriqué après coup sans
    // qu'il l'apprenne : c'est ce renversement qui ferme la classe de défauts
    // des rapports orphelins.
    // **L'aiguillage** (carte 555). Deux cas métier, deux traitements, décidés
    // ici — et non enfouis dans l'adapter, où rien ne les nommait.
    if let Some(mr_id) = rapport_de_la_rencontre(&state, &form).await {
        // Le match est **déjà au calendrier** : rien à créer, le brouillon
        // existe depuis le tirage. Le POST du coach vaut confirmation de sa
        // sélection — sans quoi il repartirait sur un brouillon non confirmé,
        // et devrait revalider ce qu'il vient d'envoyer.
        if let Err(refus) = demarrer_le_brouillon(&state, &mr_id, user.id, entry_mode).await {
            return refus;
        }
        return ouvrir(&space_id, &mr_id);
    }

    let pairing_id = match programmer_la_rencontre(&state, &space_id, &form).await {
        Ok(id) => id,
        Err(refus) => return refus,
    };

    let cmd = match construire_commande(&space_id, &form, user.id, pairing_id, entry_mode) {
        Ok(cmd) => cmd,
        Err(refus) => return refus,
    };
    creer_le_rapport(cmd, &state, &space_id).await
}

/// Démarre le brouillon d'un match du calendrier, avec le mode choisi par
/// celui qui le démarre (cartes 631 et 632).
async fn demarrer_le_brouillon(
    state: &AppState,
    mr_id: &str,
    started_by: crate::app::shared_kernel::identity::ids::CoachId,
    entry_mode: EntryMode,
) -> Result<(), Response> {
    match create_match_report_use_case::confirmer_si_brouillon(
        mr_id,
        started_by,
        entry_mode,
        state.match_report.match_report_repo.as_ref(),
        &state.match_report.event_bus,
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(create_match_report_use_case::CreateMatchReportError::Conflict) => {
            Err(conflict_response())
        }
        Err(e) => {
            tracing::error!("create_match_report: confirmation de {mr_id} : {e:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// Le rapport de cette rencontre, s'il en existe déjà un.
///
/// C'est l'aiguillage entre les deux cas métier : un rapport trouvé signifie
/// que la rencontre est au calendrier, donc que `pairing_created_listener` a
/// déjà fait son travail. Rien à créer — il n'y a qu'à ouvrir.
///
/// La requête interroge les deux camps : le coach saisit « son » match sans
/// savoir qui le calendrier a désigné comme recevant.
async fn rapport_de_la_rencontre(state: &AppState, form: &CreateMatchReportForm) -> Option<String> {
    state
        .match_report
        .match_report_repo
        .find_id_by_round_and_teams(&form.round_id, &form.home_team_id, &form.away_team_id)
        .await
        .ok()
        .flatten()
}

fn ouvrir(space_id: &str, match_report_id: &str) -> Response {
    Redirect::to(
        &AppRoutes::default()
            .match_report
            .edit_match_report(space_id, match_report_id),
    )
    .into_response()
}

/// Carte 550 — la garde serveur. Retirer les entrées de menu cache la fonction ;
/// ça n'empêche pas d'appeler la route à la main, et sans ce refus le réglage
/// serait décoratif.
///
/// La question porte sur la saison **choisie dans le formulaire**, et non sur
/// l'espace : c'est le seul endroit où elle a une réponse exacte.
async fn garde_hors_calendrier(
    state: &AppState,
    form: &CreateMatchReportForm,
) -> Result<(), Response> {
    if state
        .match_report
        .competition_data
        .autorise_hors_calendrier(&form.season_id)
        .await
    {
        return Ok(());
    }
    tracing::info!(
        season_id = %form.season_id,
        "création manuelle refusée : la saison interdit les matchs hors calendrier"
    );
    Err(StatusCode::FORBIDDEN.into_response())
}

/// Programme la rencontre au calendrier et rend son appariement (carte 552).
///
/// Le refus d'engagement (carte 551) revient **dans le formulaire**, et non en
/// code sec : c'est une erreur de saisie, que le coach doit pouvoir corriger
/// sans perdre son écran. Le layout accepte le swap sur 422.
async fn programmer_la_rencontre(
    state: &AppState,
    space_id: &str,
    form: &CreateMatchReportForm,
) -> Result<String, Response> {
    state
        .match_report
        .competition_data
        .creer_appariement(
            space_id,
            &form.competition_id,
            &form.season_id,
            &form.round_id,
            &form.home_team_id,
            &form.away_team_id,
        )
        .await
        .map_err(|e| refus_de_programmation(space_id, e))
}

fn refus_de_programmation(space_id: &str, e: CreationAppariementError) -> Response {
    let message = match e {
        CreationAppariementError::DejaEngagee { equipe, adversaire } => {
            format!("{equipe} affronte déjà {adversaire} lors de cette journée.")
        }
        CreationAppariementError::NonEnrolee(noms) => format!(
            "Équipe(s) non inscrite(s) à cette saison : {}.",
            noms.join(", ")
        ),
        CreationAppariementError::Indisponible(motif) => {
            tracing::error!("create_match_report: programmation impossible : {motif}");
            "La rencontre n'a pas pu être programmée.".to_string()
        }
    };
    formulaire_en_erreur(space_id, message)
}

fn formulaire_en_erreur(space_id: &str, message: String) -> Response {
    let gabarit = MatchSelectionTemplate {
        app_routes: Default::default(),
        widget_url: build_widget_url(space_id),
        team_widget_url: build_team_widget_url(space_id),
        space_id: space_id.to_string(),
        is_prefilled: false,
        error_message: Some(message),
        form_action: AppRoutes::default().match_report.new_match_report(space_id),
        solo_entry_value: EntryMode::SoloEntry.as_str(),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, gabarit).into_response()
}

/// Le parsing de la requête, et rien d'autre — la commande porte désormais
/// **toujours** son appariement.
fn construire_commande(
    space_id: &str,
    form: &CreateMatchReportForm,
    created_by: crate::app::shared_kernel::identity::ids::CoachId,
    pairing_id: String,
    entry_mode: EntryMode,
) -> Result<create_match_report_use_case::CreateMatchReportCommand, Response> {
    let mauvaise_requete = || StatusCode::BAD_REQUEST.into_response();
    Ok(create_match_report_use_case::CreateMatchReportCommand {
        space_id: SpaceId::try_new(space_id).map_err(|_| mauvaise_requete())?,
        competition_id: CompetitionId::try_new(&form.competition_id)
            .map_err(|_| mauvaise_requete())?,
        season_id: SeasonId::try_new(&form.season_id).map_err(|_| mauvaise_requete())?,
        round_id: RoundId::try_new(&form.round_id).map_err(|_| mauvaise_requete())?,
        home_team_id: TeamId::try_new(&form.home_team_id).map_err(|_| mauvaise_requete())?,
        away_team_id: TeamId::try_new(&form.away_team_id).map_err(|_| mauvaise_requete())?,
        created_by,
        // `Manual` porte ici une seule chose : **le coach a déjà choisi**, donc
        // la sélection est confirmée dans la foulée. Ce n'est plus une origine
        // au sens de la carte 552 — le rapport naît toujours d'un appariement,
        // et il en porte l'identifiant juste en dessous.
        origin: MatchReportOrigin::Manual,
        pairing_id: Some(pairing_id),
        entry_mode,
    })
}

/// Crée le rapport de la rencontre qu'on vient de programmer.
///
/// **Le contrôleur le crée lui-même, et il est le seul à le faire sur ce
/// chemin** (carte 555). L'appariement a été annoncé par
/// `OutOfSchedulePairingCreated`, que `pairing_created_listener` n'écoute pas :
/// personne d'autre ne s'en mêle, donc rien à dédupliquer et rien à attendre.
async fn creer_le_rapport(
    cmd: create_match_report_use_case::CreateMatchReportCommand,
    state: &AppState,
    space_id: &str,
) -> Response {
    match create_match_report_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        &state.match_report.event_bus,
    )
    .await
    {
        Ok(mr_id) => ouvrir(space_id, &mr_id.to_string()),
        Err(create_match_report_use_case::CreateMatchReportError::SameTeam) => {
            formulaire_en_erreur(
                space_id,
                "Une équipe ne peut pas s'affronter elle-même.".into(),
            )
        }
        // Deux « Commencer » simultanés : 409 et rechargement, le premier
        // l'emporte (carte 631, sur le modèle de la carte 623).
        Err(create_match_report_use_case::CreateMatchReportError::Conflict) => conflict_response(),
        Err(e) => {
            tracing::error!("create_match_report: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Démarre un rapport existant : la confirmation de sa sélection, avec le mode
/// choisi par celui qui démarre.
///
/// Pour un match du calendrier, **les équipes envoyées sont ignorées**, pour
/// tous, admin compris (carte 633, R4) — c'est le domaine qui en décide, par
/// `MatchReportDraft::keeps_its_teams`. Ignorer plutôt que refuser : un corps
/// trafiqué produit alors exactement ce qu'aurait produit un corps honnête.
pub async fn update_match_selection(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<UpdateMatchSelectionForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let cmd = match allowed_update_command(&state, &match_report_id, &form, user.id).await {
        Ok(cmd) => cmd,
        Err(refus) => return refus,
    };
    let result = update_match_selection_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        &state.match_report.event_bus,
    )
    .await;
    update_response(result, &space_id, &match_report_id)
}

/// La commande du POST, puis le droit de choisir son mode pour les équipes du
/// rapport (§ 12.9 de E22) : 400 avant tout effet dans les deux cas.
async fn allowed_update_command(
    state: &AppState,
    match_report_id: &str,
    form: &UpdateMatchSelectionForm,
    confirmed_by: crate::app::shared_kernel::identity::ids::CoachId,
) -> Result<update_match_selection_use_case::UpdateMatchSelectionCommand, Response> {
    let cmd = build_update_command(match_report_id, form, confirmed_by)?;
    let field = form.entry_mode.as_deref();
    match mode_du_rapport(state, match_report_id, field).await {
        Some(_) => Ok(cmd),
        None => Err(StatusCode::BAD_REQUEST.into_response()),
    }
}

/// Le parsing du POST, et rien d'autre : le mode d'abord, refusé en 400 avant
/// tout effet s'il est inconnu (carte 632).
pub(crate) fn build_update_command(
    match_report_id: &str,
    form: &UpdateMatchSelectionForm,
    confirmed_by: crate::app::shared_kernel::identity::ids::CoachId,
) -> Result<update_match_selection_use_case::UpdateMatchSelectionCommand, Response> {
    let mauvaise_requete = || StatusCode::BAD_REQUEST.into_response();
    let entry_mode = read_entry_mode(form.entry_mode.as_deref()).map_err(|_| mauvaise_requete())?;
    Ok(
        update_match_selection_use_case::UpdateMatchSelectionCommand {
            match_report_id: MatchReportId::try_new(match_report_id)
                .map_err(|_| mauvaise_requete())?,
            requested_teams: requested_teams(form).map_err(|_| mauvaise_requete())?,
            confirmed_by,
            entry_mode,
        },
    )
}

/// Les deux équipes du formulaire, quand il les porte toutes les deux. Le use
/// case décide si elles comptent.
fn requested_teams(form: &UpdateMatchSelectionForm) -> Result<Option<(TeamId, TeamId)>, ()> {
    let (Some(home), Some(away)) = (&form.home_team_id, &form.away_team_id) else {
        return Ok(None);
    };
    let home = TeamId::try_new(home).map_err(|_| ())?;
    let away = TeamId::try_new(away).map_err(|_| ())?;
    Ok(Some((home, away)))
}

/// Deux « Commencer » simultanés : le second échoue sur la version, ou trouve
/// le match déjà démarré — 409 et rechargement dans les deux cas, et il arrive
/// sur l'état du match (cartes 623, 631, 633).
fn update_response(
    result: Result<MatchReportId, update_match_selection_use_case::UpdateMatchSelectionError>,
    space_id: &str,
    match_report_id: &str,
) -> Response {
    use update_match_selection_use_case::UpdateMatchSelectionError as E;
    match result {
        Ok(_) => Redirect::to(
            &AppRoutes::default()
                .match_report
                .step2(space_id, match_report_id),
        )
        .into_response(),
        Err(E::SameTeam) => StatusCode::BAD_REQUEST.into_response(),
        Err(E::TeamNotAvailable(tid)) => {
            tracing::warn!("update_match_selection: team {tid} not available");
            StatusCode::CONFLICT.into_response()
        }
        Err(E::Conflict | E::NotInDraftPhase) => conflict_response(),
        Err(e) => {
            tracing::error!("update_match_selection: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod entry_mode_tests {
    use super::*;

    /// La valeur de l'option « Je saisis tout » se relit telle quelle.
    #[test]
    fn the_solo_entry_value_reads_as_solo_entry() {
        assert_eq!(
            read_entry_mode(Some(EntryMode::SoloEntry.as_str())),
            Ok(EntryMode::SoloEntry)
        );
    }

    /// « Chacun son équipe » se lit depuis la carte 652, même si l'option
    /// reste grisée jusqu'à la 659.
    #[test]
    fn the_team_entry_value_reads_as_team_entry() {
        assert_eq!(
            read_entry_mode(Some(EntryMode::TeamEntry.as_str())),
            Ok(EntryMode::TeamEntry)
        );
    }

    /// Une valeur inconnue est refusée : le handler répond 400.
    #[test]
    fn an_unknown_value_is_refused() {
        assert_eq!(read_entry_mode(Some("UnknownEntry")), Err(()));
        assert_eq!(read_entry_mode(Some("")), Err(()));
    }

    /// Un POST sans le champ démarre en « Je saisis tout » (décision 2 du
    /// cadrage E21).
    #[test]
    fn a_missing_field_defaults_to_solo_entry() {
        assert_eq!(read_entry_mode(None), Ok(EntryMode::SoloEntry));
    }
}

/// Carte 633 — démarrer depuis le calendrier sans sélection.
#[cfg(test)]
mod start_from_schedule_tests {
    use super::*;
    use crate::app::match_report::use_cases::test_doubles::{
        PlainCompetition, ReadyTeams, RecordingRepository,
    };
    use crate::app::shared_kernel::identity::ids::CoachId;

    fn draft(origin: MatchReportOrigin) -> MatchReportDraft {
        MatchReportDraft::create(
            MatchReportId::new(),
            SpaceId::new(),
            CompetitionId::new(),
            SeasonId::new(),
            RoundId::new(),
            TeamId::new(),
            TeamId::new(),
            CoachId::new(),
            origin,
            None,
        )
        .expect("deux équipes distinctes")
        .0
    }

    /// Ouvre la page d'un brouillon ; rend le statut, le corps, et le nombre
    /// d'événements appendus entre-temps.
    async fn open(origin: MatchReportOrigin) -> (StatusCode, String, usize) {
        let d = draft(origin);
        let mr_id = d.id.to_string();
        let repo = RecordingRepository::holding(MatchReportState::Draft(d));
        let ports = EditPagePorts {
            repo: &repo,
            competition_data: &PlainCompetition,
            team_data: &ReadyTeams,
        };
        let response = edit_page(&ports, "space", &mr_id).await;
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        let body = String::from_utf8(bytes.to_vec()).expect("UTF-8");
        (status, body, repo.appended().len())
    }

    /// R3 : ouvrir le lien d'un match du calendrier montre « Qui saisit ? »,
    /// sans sélection d'équipes, et **ne confirme rien**.
    #[tokio::test]
    async fn opening_a_scheduled_draft_shows_the_mode_and_confirms_nothing() {
        let (status, body, appended) = open(MatchReportOrigin::Pairing).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Qui saisit ce match ?"), "{body}");
        assert!(body.contains("Commencer le match"), "{body}");
        assert!(
            !body.contains("teams-container"),
            "pas de sélection d'équipes"
        );
        assert_eq!(appended, 0, "le GET ne doit rien écrire");
    }

    /// Un brouillon hors calendrier d'avant la carte 552 garde la sélection.
    #[tokio::test]
    async fn opening_a_manual_draft_keeps_the_selection() {
        let (status, body, appended) = open(MatchReportOrigin::Manual).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("teams-container"), "{body}");
        assert_eq!(appended, 0);
    }

    fn form(home: Option<&str>, away: Option<&str>) -> UpdateMatchSelectionForm {
        UpdateMatchSelectionForm {
            home_team_id: home.map(str::to_string),
            away_team_id: away.map(str::to_string),
            entry_mode: None,
        }
    }

    /// Le formulaire de « Qui saisit ? » n'a pas de champ d'équipe : la
    /// commande part sans, et le brouillon démarre avec les siennes.
    #[test]
    fn a_post_without_teams_requests_none() {
        let mr_id = MatchReportId::new().to_string();
        let cmd = build_update_command(&mr_id, &form(None, None), CoachId::new())
            .unwrap_or_else(|_| panic!("le corps sans équipes doit être accepté"));
        assert!(cmd.requested_teams.is_none());
    }

    /// Des équipes envoyées passent dans la commande : c'est le use case qui
    /// les écarte pour un match du calendrier (cf. ses tests).
    #[test]
    fn a_post_with_teams_passes_them_to_the_use_case() {
        let (home, away) = (TeamId::new().to_string(), TeamId::new().to_string());
        let mr_id = MatchReportId::new().to_string();
        let cmd = build_update_command(&mr_id, &form(Some(&home), Some(&away)), CoachId::new())
            .unwrap_or_else(|_| panic!("deux équipes valides doivent être acceptées"));
        assert!(cmd.requested_teams.is_some());
    }

    /// Deux « Commencer » simultanés : le second, qui trouve le match déjà
    /// démarré, reçoit 409 et recharge la page.
    #[test]
    fn a_second_start_is_a_conflict() {
        let response = update_response(
            Err(update_match_selection_use_case::UpdateMatchSelectionError::NotInDraftPhase),
            "space",
            "mr",
        );
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
