use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{D3Roll, DedicatedFans, EntryMode};
use crate::app::match_report::io::web::mvp_step_controller::side_slug;
use crate::app::match_report::io::web::write_conflict::conflict_response;
use crate::app::match_report::ports::TeamInfoDto;
use crate::app::match_report::use_cases::match_report_access_service::{
    arrival_side, PorteeRapport,
};
use crate::app::match_report::use_cases::record_fan_factor_use_case::{
    self, RecordFanFactorError, RecordFanFactorOutcome,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

// ── Template ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "pre-match.html")]
pub struct PreMatchTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_initials: String,
    pub away_initials: String,
    pub home_coach_name: String,
    pub away_coach_name: String,
    pub home_roster_name: String,
    pub away_roster_name: String,
    pub home_logo_url: Option<String>,
    pub away_logo_url: Option<String>,
    pub home_team_context_url: String,
    pub away_team_context_url: String,
    pub form_action: String,
    pub fan_factor_already_recorded: bool,
    /// Les jets enregistrés, s'il y en a (carte 494).
    ///
    /// Le gabarit en fait la valeur initiale des deux champs — vides sinon.
    /// Ils valaient 2 en dur, si bien qu'un rapport vierge proposait un jet
    /// qu'on n'avait pas fait, et qu'un rapport déjà saisi affichait 2 au lieu
    /// du sien, sous un bandeau « déjà enregistré » qui disait le contraire.
    pub home_fan_roll: Option<u8>,
    pub away_fan_roll: Option<u8>,
    /// Les valeurs d'équipe figées par le premier fan factor de « Chacun son
    /// équipe » (R4 de E22, carte 654), lues dans l'agrégat. `None` en « Je
    /// saisis tout » ou tant qu'aucun jet n'est lancé : la page affiche alors
    /// les valeurs du moment, comme avant.
    pub frozen_values: Option<FrozenTeamValuesVm>,
    /// L'avant-match d'un camp, en « Chacun son équipe » (carte 654). `None`
    /// en « Je saisis tout ».
    pub team_entry: Option<TeamEntryVm>,
}

/// Les valeurs d'équipe figées, en kPo.
pub struct FrozenTeamValuesVm {
    pub home: u32,
    pub away: u32,
}

impl FrozenTeamValuesVm {
    /// `Some` dès que les deux valeurs sont dans l'agrégat.
    pub fn from_domain(pm: &MatchReportPreMatch) -> Option<Self> {
        Some(Self {
            home: pm.home_team_value?.into_inner(),
            away: pm.away_team_value?.into_inner(),
        })
    }
}

/// Le camp que saisit la page, en « Chacun son équipe » (carte 654).
pub struct TeamEntryVm {
    /// `home` ou `away` : nomme aussi les données Alpine du camp.
    pub side: &'static str,
    pub own_team_name: String,
    /// Le jet du camp et son fan factor, une fois lancé : la page ne propose
    /// plus de le saisir, le domaine refusant un second jet.
    pub own_roll: Option<u8>,
    pub own_total: Option<u32>,
    /// Le bloc de l'adversaire, rechargé sur le signal du match.
    pub opponent_url: String,
    /// La suite, une fois le jet lancé : coups de pouce ou match.
    pub next_url: Option<String>,
    /// L'autre camp, proposé à l'admin seul (§ 12.6 de E22).
    pub switch: Option<SwitchSideVm>,
}

/// Le lien vers l'avant-match de l'autre camp.
pub struct SwitchSideVm {
    pub url: String,
    pub team_name: String,
}

impl PreMatchTemplate {
    /// Ce que les deux modes partagent : les équipes, leurs liens, sans jet ni
    /// camp.
    pub fn base(
        space_id: &str,
        match_report_id: &str,
        (home_id, away_id): (String, String),
        home_info: TeamInfoDto,
        away_info: TeamInfoDto,
    ) -> Self {
        let base_url = AppRoutes::default().teams.team_match_context_json(space_id);
        Self {
            app_routes: Default::default(),
            space_id: space_id.to_string(),
            match_report_id: match_report_id.to_string(),
            home_initials: initials_from(&home_info.team_name),
            away_initials: initials_from(&away_info.team_name),
            home_team_name: home_info.team_name,
            away_team_name: away_info.team_name,
            home_coach_name: home_info.coach_name,
            away_coach_name: away_info.coach_name,
            home_roster_name: home_info.roster_name,
            away_roster_name: away_info.roster_name,
            home_logo_url: home_info.logo_url,
            away_logo_url: away_info.logo_url,
            home_team_context_url: format!("{}?team_id={}", base_url, home_id),
            away_team_context_url: format!("{}?team_id={}", base_url, away_id),
            home_team_id: home_id,
            away_team_id: away_id,
            form_action: AppRoutes::default()
                .match_report
                .step2(space_id, match_report_id),
            fan_factor_already_recorded: false,
            home_fan_roll: None,
            away_fan_roll: None,
            frozen_values: None,
            team_entry: None,
        }
    }
}

impl IntoResponse for PreMatchTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

// ── GET ───────────────────────────────────────────────────────────────────────

pub async fn get_pre_match(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let mr_state = match state
        .match_report
        .match_report_repo
        .find_by_id(&match_report_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_pre_match find_by_id {match_report_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // « Chacun son équipe » : chacun son avant-match (carte 654).
    if let MatchReportState::PreMatch(pm) = &mr_state {
        if pm.entry_mode == EntryMode::TeamEntry {
            return redirect_to_own_side(&state, &user, (&space_id, &match_report_id), pm).await;
        }
    }

    // Les trois états portent les jets. `ReadyToPublish` et `Published` rendaient
    // `true` en dur : c'est vrai de l'enregistrement, mais ça ne dit pas ce qui
    // a été enregistré — et le gabarit en a besoin.
    let (home_id, away_id, home_fan_roll, away_fan_roll) = match mr_state {
        MatchReportState::PreMatch(pm) => (
            pm.home_team_id.to_string(),
            pm.away_team_id.to_string(),
            pm.home_fan_roll,
            pm.away_fan_roll,
        ),
        MatchReportState::ReadyToPublish(rtp) => (
            rtp.home_team_id.to_string(),
            rtp.away_team_id.to_string(),
            rtp.home_fan_roll,
            rtp.away_fan_roll,
        ),
        MatchReportState::Published(p) => (
            p.home_team_id.to_string(),
            p.away_team_id.to_string(),
            p.home_fan_roll,
            p.away_fan_roll,
        ),
        MatchReportState::Draft(_) => {
            let url = AppRoutes::default()
                .match_report
                .edit_match_report(&space_id, &match_report_id);
            return Redirect::to(&url).into_response();
        }
        MatchReportState::Cancelled(_) => return StatusCode::GONE.into_response(),
    };
    let fan_factor_already_recorded = home_fan_roll.is_some() && away_fan_roll.is_some();

    let (home_info, away_info) = tokio::join!(
        state.match_report.team_data.find_team_info(&home_id),
        state.match_report.team_data.find_team_info(&away_id),
    );
    let home_info = home_info.unwrap_or_default();
    let away_info = away_info.unwrap_or_default();

    PreMatchTemplate {
        fan_factor_already_recorded,
        home_fan_roll: home_fan_roll.map(|r| r.value()),
        away_fan_roll: away_fan_roll.map(|r| r.value()),
        ..PreMatchTemplate::base(
            &space_id,
            &match_report_id,
            (home_id, away_id),
            home_info,
            away_info,
        )
    }
    .into_response()
}

/// En « Chacun son équipe », `/step2` mène à l'avant-match du camp de
/// l'utilisateur (carte 654) ; l'admin arrive au domicile.
async fn redirect_to_own_side(
    state: &AppState,
    user: &User,
    (space_id, match_report_id): (&str, &str),
    pm: &MatchReportPreMatch,
) -> Response {
    let scope = PorteeRapport {
        competition_id: pm.competition_id.to_string(),
        home_team_id: pm.home_team_id.to_string(),
        away_team_id: pm.away_team_id.to_string(),
    };
    let team_data = state.match_report.team_data.as_ref();
    let side = arrival_side(team_data, &scope, &user.id.to_string()).await;
    let url =
        AppRoutes::default()
            .match_report
            .step2_side(space_id, match_report_id, side_slug(side));
    Redirect::to(&url).into_response()
}

// ── POST ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RecordFanFactorForm {
    pub home_fan_roll: u8,
    pub away_fan_roll: u8,
}

pub async fn post_pre_match(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordFanFactorForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let home_fan_roll = match D3Roll::try_new(form.home_fan_roll) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let away_fan_roll = match D3Roll::try_new(form.away_fan_roll) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let mr_id = match MatchReportId::try_new(&match_report_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = record_fan_factor_use_case::RecordFanFactorCommand {
        match_report_id: mr_id,
        home_fan_roll,
        away_fan_roll,
        home_dedicated_fans: DedicatedFans::default(),
        away_dedicated_fans: DedicatedFans::default(),
        recorded_by: user.id,
    };

    let outcome = record_fan_factor_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        state.match_report.competition_data.as_ref(),
    )
    .await;

    match outcome {
        Ok(outcome) => {
            Redirect::to(&next_url(&space_id, &match_report_id, outcome)).into_response()
        }
        Err(e) => fan_factor_error_response(e),
    }
}

/// La suite d'un fan factor, dans les deux modes : la page de coups de pouce
/// que le use case désigne, ou le match.
pub(crate) fn next_url(
    space_id: &str,
    match_report_id: &str,
    outcome: RecordFanFactorOutcome,
) -> String {
    let routes = AppRoutes::default().match_report;
    match outcome {
        RecordFanFactorOutcome::RedirectToInducements { team_id } => {
            routes.inducements(space_id, match_report_id, &team_id)
        }
        RecordFanFactorOutcome::RedirectToLiveMatch => routes.live(space_id, match_report_id),
    }
}

/// Les refus d'un fan factor, dans les deux modes. Un refus du domaine — les
/// deux jets d'un coup en « Chacun son équipe », un second jet du même camp —
/// se dit en 422, avec son motif (carte 654).
pub(crate) fn fan_factor_error_response(e: RecordFanFactorError) -> Response {
    match e {
        RecordFanFactorError::NotFound => StatusCode::NOT_FOUND.into_response(),
        RecordFanFactorError::NotInPreMatchPhase => StatusCode::CONFLICT.into_response(),
        RecordFanFactorError::Conflict => conflict_response(),
        RecordFanFactorError::Domain(err) => domain_refusal(err),
        other => {
            tracing::error!("fan factor : {other:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn domain_refusal(err: DomainError) -> Response {
    tracing::warn!("fan factor refusé : {err}");
    (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un gabarit minimal : seuls les deux jets varient d'un cas à l'autre.
    fn gabarit(home: Option<u8>, away: Option<u8>) -> PreMatchTemplate {
        PreMatchTemplate {
            app_routes: AppRoutes::default(),
            space_id: "s".into(),
            match_report_id: "m".into(),
            home_team_id: "t-home".into(),
            away_team_id: "t-away".into(),
            home_team_name: "Domicile".into(),
            away_team_name: "Visiteur".into(),
            home_initials: "DO".into(),
            away_initials: "VI".into(),
            home_coach_name: "A".into(),
            away_coach_name: "B".into(),
            home_roster_name: "R".into(),
            away_roster_name: "R".into(),
            home_logo_url: None,
            away_logo_url: None,
            home_team_context_url: "/ctx/home".into(),
            away_team_context_url: "/ctx/away".into(),
            form_action: "/post".into(),
            fan_factor_already_recorded: home.is_some() && away.is_some(),
            home_fan_roll: home,
            away_fan_roll: away,
            frozen_values: None,
            team_entry: None,
        }
    }

    /// **Le défaut d'origine.** Les deux champs portaient `value="2"` et
    /// l'`x-data` initialisait ses jets à 2 : un rapport neuf proposait un jet
    /// qu'on n'avait pas fait, et rien ne distinguait cette valeur d'une saisie.
    #[test]
    fn un_rapport_vierge_ne_propose_aucun_jet() {
        let html = gabarit(None, None).render().unwrap();
        assert!(
            !html.contains(r#"value="2""#),
            "aucun jet ne doit être proposé par défaut"
        );
        assert!(html.contains("homeRoll: ''"), "le jet domicile part vide");
        assert!(html.contains("awayRoll: ''"), "le jet visiteur part vide");
    }

    /// L'autre moitié : ce qui a été enregistré revient à l'écran. Les deux
    /// valeurs diffèrent à dessein — égales, le test passerait en confondant
    /// les deux champs.
    #[test]
    fn un_rapport_saisi_reaffiche_ses_jets() {
        let html = gabarit(Some(1), Some(3)).render().unwrap();
        assert!(html.contains("homeRoll: 1"), "le jet domicile enregistré");
        assert!(html.contains("awayRoll: 3"), "le jet visiteur enregistré");
        assert!(
            html.contains("déjà enregistré"),
            "le bandeau accompagne les jets"
        );
    }

    /// L'avant-match d'un camp, en « Chacun son équipe » (carte 654).
    fn camp(own_roll: Option<u8>) -> PreMatchTemplate {
        PreMatchTemplate {
            team_entry: Some(TeamEntryVm {
                side: "away",
                own_team_name: "Visiteur".into(),
                own_roll,
                own_total: own_roll.map(|r| 1 + u32::from(r)),
                opponent_url: "/step2/home/fan-factor".into(),
                next_url: own_roll.map(|_| "/inducements/t-away".into()),
                switch: None,
            }),
            ..gabarit(None, own_roll)
        }
    }

    /// Le seul jet du camp : aucun champ pour celui de l'autre, qui arrive
    /// par le widget.
    #[test]
    fn un_camp_ne_saisit_que_son_jet() {
        let html = camp(None).render().unwrap();
        assert!(html.contains(r#"name="fan_roll""#));
        assert!(!html.contains(r#"name="home_fan_roll""#));
        assert!(!html.contains(r#"name="away_fan_roll""#));
        assert!(html.contains(r#"x-model="awayRoll""#));
        assert!(html.contains(r#"hx-get="/step2/home/fan-factor""#));
        assert!(html.contains("matchChanged[detail.id=='m'] from:body"));
    }

    /// Le signal ne va qu'aux présents : la page de camp se déclare, celle de
    /// « Je saisis tout » non (décision 5 de la carte 654).
    #[test]
    fn seule_la_page_de_camp_declare_sa_presence() {
        assert!(camp(None)
            .render()
            .unwrap()
            .contains(r#"data-activity="write""#));
        assert!(!gabarit(None, None)
            .render()
            .unwrap()
            .contains(r#"data-activity="write""#));
    }

    /// Un second jet serait refusé : le jet lancé ne se ressaisit pas, la
    /// page mène à la suite.
    #[test]
    fn un_jet_lance_mene_a_la_suite() {
        let html = camp(Some(2)).render().unwrap();
        assert!(!html.contains(r#"name="fan_roll""#));
        assert!(html.contains(r#"href="/inducements/t-away""#));
        assert!(html.contains("Votre fan factor est enregistré"));
    }

    /// L'admin, seul à pouvoir saisir l'autre camp, s'y voit proposer le lien.
    #[test]
    fn l_admin_peut_saisir_l_autre_camp() {
        let mut page = camp(None);
        if let Some(te) = page.team_entry.as_mut() {
            te.switch = Some(SwitchSideVm {
                url: "/step2/home".into(),
                team_name: "Domicile".into(),
            });
        }
        let html = page.render().unwrap();
        assert!(html.contains(r#"href="/step2/home""#));
        assert!(html.contains("Saisir pour Domicile"));
    }

    /// R4 : les valeurs figées viennent de l'agrégat (décision 4).
    #[test]
    fn les_valeurs_figees_viennent_de_l_agregat() {
        let pm = crate::app::match_report::use_cases::test_doubles::pre_match();
        let frozen = FrozenTeamValuesVm::from_domain(&pm);
        let html = PreMatchTemplate {
            frozen_values: frozen,
            ..camp(None)
        }
        .render()
        .unwrap();
        assert!(
            html.contains("frozen: { home: 1000, away: 1000 }"),
            "{html}"
        );
        assert!(gabarit(None, None)
            .render()
            .unwrap()
            .contains("frozen: null"));
    }

    #[test]
    fn sans_valeurs_dans_l_agregat_rien_n_est_fige() {
        let mut pm = crate::app::match_report::use_cases::test_doubles::pre_match();
        pm.away_team_value = None;
        assert!(FrozenTeamValuesVm::from_domain(&pm).is_none());
    }

    /// **`formatKpo` recomposait le nombre** depuis les milliers et les
    /// centaines, et le chiffre des dizaines n'apparaissait dans aucune de ses
    /// deux branches : 2075 s'affichait « 2 000 kPo ». Le gabarit ne doit plus
    /// porter cette arithmétique.
    #[test]
    fn le_gabarit_ne_recompose_plus_les_montants() {
        let html = gabarit(None, None).render().unwrap();
        assert!(
            !html.contains("Math.floor(v / 1000)"),
            "le montant s'imprime, il ne se recompose pas"
        );
        assert!(!html.contains("'00 kPo'"), "plus de centaines recollées");
        assert!(
            html.contains("return v + ' kPo';"),
            "la valeur, telle quelle"
        );
    }
}
