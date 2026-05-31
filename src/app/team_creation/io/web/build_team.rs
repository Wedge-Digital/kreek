use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use crate::app::references::domain::models::Team as RefTeam;
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::references::io::web::pickers::{
    build_player_positions, build_roster_items_with_tiers,
    HiredPlayerRowVm, PlayerPositionVm, RosterPickerItemWithTier,
};
use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::shared_kernel::common_types::{Entity, EntityId};
use crate::app::shared_kernel::staff::{StaffId, StaffKind, StaffMaxQuantity, StaffName, StaffPrice};
use crate::app::team_creation::domain::roster::{
    CrossLimit, PlayerId, PlayerDefinition, PlayerMaxQuantity, PlayerName,
    PlayerPrice, RerollBasePrice, Roster, RosterId, RosterName,
};
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::app::team_creation::domain::team_roster_selected::{MAX_REROLL_COUNT, RosterSelectedTeam};
use crate::app::team_creation::domain::team_staff::TeamStaff;
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::app::team_creation::use_cases::commands::{
    BuyRerollCommand, BuyStaffCommand, FirePlayerCommand, HirePlayerCommand,
    RemoveRerollCommand, RemoveStaffCommand, SubmitTeamCommand,
};
use crate::app::team_creation::use_cases::buy_reroll   as buy_reroll_uc;
use crate::app::team_creation::use_cases::buy_staff    as buy_staff_uc;
use crate::app::team_creation::use_cases::fire_player  as fire_uc;
use crate::app::team_creation::use_cases::hire_player  as hire_uc;
use crate::app::team_creation::use_cases::remove_reroll as remove_reroll_uc;
use crate::app::team_creation::use_cases::remove_staff  as remove_staff_uc;
use crate::app::team_creation::use_cases::submit_team   as submit_uc;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;

// ── View models ───────────────────────────────────────────────────────────────

pub struct StaffRowVm {
    pub id:            String,
    pub name:          String,
    pub price:         u32,
    pub max_qty_label: String,
    pub max_qty:       usize,
    pub quantity:      usize,
    pub line_cost:     u32,
}

pub struct RerollVm {
    pub price:     u32,
    pub quantity:  u8,
    pub max:       u8,
    pub line_cost: u32,
}

pub struct CartLineVm {
    pub name:  String,
    pub qty:   usize,
    pub total: u32,
}

pub struct CartVm {
    pub player_lines:    Vec<CartLineVm>,
    pub player_subtotal: u32,
    pub reroll_qty:      u8,
    pub reroll_cost:     u32,
    pub staff_lines:       Vec<CartLineVm>,
    pub staff_subtotal:    u32,
    pub total:             u32,
    pub budget:            u32,
    pub remaining:         u32,
    pub progress_pct:      u32,
}

// ── Roster builder: references → domain ──────────────────────────────────────

fn staff_kind(uid: &str) -> StaffKind {
    match uid {
        "APOTHECARY"       => StaffKind::Apothecary,
        "CHEERLEADERS"     => StaffKind::Cheerleaders,
        "COACH_ASSISTANTS" => StaffKind::CoachAssistant,
        _                  => StaffKind::CoachAssistant,
    }
}

pub fn build_roster_from_ref(ref_team: &RefTeam, ref_repo: &dyn IReferenceRepository) -> Roster {
    let player_definitions = ref_team.available_players.iter().map(|p| PlayerDefinition {
        id:           PlayerId(p.uid.clone()),
        name:         PlayerName(p.position_name.clone()),
        max_quantity: PlayerMaxQuantity(p.max_quantity),
        price:        PlayerPrice(p.cost / 1000),
    }).collect();

    let allowed_staff = ref_team.allowed_staff.iter().filter_map(|uid| {
        ref_repo.list_staff().iter().find(|s| s.uid == *uid).map(|s| TeamStaff {
            id:           StaffId(s.uid.clone()),
            name:         StaffName(s.name.clone()),
            price:        StaffPrice(s.price),
            max_quantity: StaffMaxQuantity(s.max_quantity as u8),
            kind:         staff_kind(&s.uid),
        })
    }).collect();

    Roster {
        id:               RosterId(ref_team.uid.clone()),
        name:             RosterName(ref_team.name.clone()),
        player_definitions,
        allowed_staff,
        cross_limits:     vec![],
        reroll_price:     RerollBasePrice(ref_team.reroll_cost / 1000),
    }
}

// ── Hired row builder ─────────────────────────────────────────────────────────

pub fn build_hired_rows(
    team:     &RosterSelectedTeam,
    ref_repo: &dyn IReferenceRepository,
) -> Vec<HiredPlayerRowVm> {
    team.roster.player_definitions.iter().map(|def| {
        let quantity      = team.hired_players().iter().filter(|p| p.id == def.id).count();
        let line_cost_kpo = quantity as u32 * def.price.0;
        let is_max        = quantity >= def.max_quantity.0 as usize
            || team.hired_players().len() >= 16;

        let (ma, st, ag, pa, av, skills) = ref_repo
            .list_teams()
            .iter()
            .flat_map(|t| t.available_players.iter().map(move |p| (t, p)))
            .find(|(_, p)| p.uid == def.id.0)
            .map(|(_, p)| {
                let to_plus = |v: u8| if v == 0 { "—".into() } else { format!("{}+", v) };
                let skills = p.skills.iter()
                    .map(|uid| ref_repo.find_skill_by_uid(uid)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| uid.clone()))
                    .collect::<Vec<_>>()
                    .join(", ");
                (p.ma, p.st, to_plus(p.ag), to_plus(p.pa), to_plus(p.av), skills)
            })
            .unwrap_or((0, 0, "?".into(), "?".into(), "?".into(), String::new()));

        HiredPlayerRowVm {
            uid:           def.id.0.clone(),
            name:          def.name.0.clone(),
            cost_kpo:      def.price.0,
            max_qty_label: format!("0-{}", def.max_quantity.0),
            ma, st, ag, pa, av, skills,
            quantity,
            line_cost_kpo,
            is_max,
        }
    }).collect()
}

// ── Staff + reroll builders ───────────────────────────────────────────────────

fn build_staff_rows(team: &RosterSelectedTeam) -> Vec<StaffRowVm> {
    team.roster.allowed_staff.iter().map(|staff| {
        let quantity  = team.hired_staff().iter().filter(|s| s.id == staff.id).count();
        let line_cost = quantity as u32 * staff.price.0;
        StaffRowVm {
            id:            staff.id.0.clone(),
            name:          staff.name.0.clone(),
            price:         staff.price.0,
            max_qty_label: format!("0-{}", staff.max_quantity.0),
            max_qty:       staff.max_quantity.0 as usize,
            quantity,
            line_cost,
        }
    }).collect()
}

fn build_reroll_vm(team: &RosterSelectedTeam) -> RerollVm {
    let price    = team.roster.reroll_price.0;
    let quantity = team.reroll_count();
    RerollVm { price, quantity, max: MAX_REROLL_COUNT, line_cost: price * quantity as u32 }
}

fn build_cart_vm(team: &RosterSelectedTeam) -> CartVm {
    let mut player_lines: Vec<CartLineVm> = Vec::new();
    for def in &team.roster.player_definitions {
        let qty = team.hired_players().iter().filter(|p| p.id == def.id).count();
        if qty > 0 {
            player_lines.push(CartLineVm {
                name:  def.name.0.clone(),
                qty,
                total: qty as u32 * def.price.0,
            });
        }
    }
    let player_subtotal: u32 = player_lines.iter().map(|l| l.total).sum();

    let reroll_qty  = team.reroll_count();
    let reroll_cost = team.roster.reroll_price.0 * reroll_qty as u32;

    let mut staff_lines: Vec<CartLineVm> = Vec::new();
    for staff in &team.roster.allowed_staff {
        let qty = team.hired_staff().iter().filter(|s| s.id == staff.id).count();
        if qty > 0 {
            staff_lines.push(CartLineVm {
                name:  staff.name.0.clone(),
                qty,
                total: qty as u32 * staff.price.0,
            });
        }
    }
    let staff_subtotal: u32 = staff_lines.iter().map(|l| l.total).sum();

    let total        = player_subtotal + staff_subtotal + reroll_cost;
    let remaining    = team.remaining_budget().unwrap_or(0);
    let budget       = total + remaining;
    let progress_pct = if budget > 0 { (total * 100 / budget).min(100) } else { 0 };

    CartVm {
        player_lines,
        player_subtotal,
        reroll_qty,
        reroll_cost,
        staff_lines,
        staff_subtotal,
        total,
        budget,
        remaining,
        progress_pct,
    }
}

// ── Rules panel view model ────────────────────────────────────────────────────

pub struct RulesTierVm {
    pub name:     String,
    pub budget:   u32,
    pub start_xp: u32,
}

pub struct RulesPanelVm {
    pub competition_name: String,
    pub season_name:      String,
    pub tiers:            Vec<RulesTierVm>,
}

// ── Helpers de réponse d'erreur HTMX ─────────────────────────────────────────

fn player_error(msg: &str) -> Response {
    Response::builder()
        .header("HX-Retarget", "#player-table-error")
        .header("HX-Reswap", "innerHTML")
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(format!(r#"<p class="table-error">{msg}</p>"#)))
        .unwrap()
}

fn staff_error(msg: &str) -> Response {
    Response::builder()
        .header("HX-Retarget", "#staff-table-error")
        .header("HX-Reswap", "innerHTML")
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(format!(r#"<p class="table-error">{msg}</p>"#)))
        .unwrap()
}

// ── Page complète ─────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "build-team.html")]
pub struct BuildTeamTemplate {
    pub web_routes:          WebRoutes,
    pub team_routes:         TeamCreationRoutes,
    pub space_id:            String,
    pub team_id:             String,
    pub rosters:             Vec<RosterPickerItemWithTier>,
    pub selected_roster_uid: Option<String>,
    pub hired_rows:          Vec<HiredPlayerRowVm>,
    pub staff_rows:          Vec<StaffRowVm>,
    pub reroll:              Option<RerollVm>,
    pub cart:                Option<CartVm>,
    pub rules_panel:         RulesPanelVm,
}

impl IntoResponse for BuildTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn build_team(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let draft = match state.team_creation.team_repository.find_by_id(&team_id_val).await {
        Ok(Some(t)) => t,
        Ok(None)    => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("build_team draft find {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let roster_team = match state.team_creation.roster_repository.find_by_id(&team_id_val).await {
        Ok(opt) => opt,
        Err(e) => {
            tracing::error!("build_team roster find {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let ref_repo = state.references.repository.as_ref();
    let rosters  = build_roster_items_with_tiers(ref_repo, draft.creation_rules());

    let (selected_roster_uid, hired_rows, staff_rows, reroll, cart) = match &roster_team {
        None => (None, vec![], vec![], None, None),
        Some(team) => {
            let uid   = team.roster.id.0.clone();
            let rows  = build_hired_rows(team, ref_repo);
            let staff = build_staff_rows(team);
            let rv    = build_reroll_vm(team);
            let cv    = build_cart_vm(team);
            (Some(uid), rows, staff, Some(rv), Some(cv))
        }
    };

    let competition_name = if let Ok(id) = EntityId::try_new(draft.competition_id()) {
        state.competitions.competition_repository
            .find_base_info(&id).await.ok().flatten()
            .map(|i| i.name).unwrap_or_default()
    } else { String::new() };

    let season_name = if let Ok(id) = EntityId::try_new(draft.season_id()) {
        state.competitions.season_repository
            .find_base_info(&id).await.ok().flatten()
            .map(|i| i.name).unwrap_or_default()
    } else { String::new() };

    let rules_panel = RulesPanelVm {
        competition_name,
        season_name,
        tiers: draft.creation_rules().tiers.iter().map(|t| RulesTierVm {
            name:     t.name.clone(),
            budget:   t.budget,
            start_xp: t.start_xp,
        }).collect(),
    };

    BuildTeamTemplate {
        web_routes:  Default::default(),
        team_routes: Default::default(),
        space_id,
        team_id,
        rosters,
        selected_roster_uid,
        hired_rows,
        staff_rows,
        reroll,
        cart,
        rules_panel,
    }.into_response()
}

// ── Fragment joueurs (sélection roster) ───────────────────────────────────────

#[derive(Template)]
#[template(path = "roster-players-fragment.html")]
pub struct RosterPlayersFragment {
    pub positions:   Vec<PlayerPositionVm>,
    pub team_routes: TeamCreationRoutes,
    pub space_id:    String,
    pub team_id:     String,
    pub cart:        Option<CartVm>,
    pub staff_rows:  Vec<StaffRowVm>,
    pub reroll:      Option<RerollVm>,
}

impl IntoResponse for RosterPlayersFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_roster_players(
    Path((space_id, team_id, roster_uid)): Path<(String, String, String)>,
    State(state):                           State<AppState>,
) -> impl IntoResponse {
    let ref_repo = state.references.repository.as_ref();

    let ref_team = match ref_repo.find_team_by_uid(&roster_uid) {
        Some(t) => t,
        None    => return StatusCode::NOT_FOUND.into_response(),
    };

    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let roster = build_roster_from_ref(ref_team, ref_repo);

    let draft: DraftTeam = match state.team_creation.team_repository.find_by_id(&team_id_val).await {
        Ok(Some(t)) => t,
        Ok(None)    => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_roster_players draft find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let roster_team: RosterSelectedTeam = match state.team_creation.roster_repository
        .find_by_id(&team_id_val).await
    {
        Ok(Some(existing)) => match existing.choose_roster(roster) {
            Ok(t)  => t,
            Err(e) => {
                tracing::warn!("choose_roster rejected: {:?}", e);
                return StatusCode::UNPROCESSABLE_ENTITY.into_response();
            }
        },
        Ok(None) => {
            let ruleset      = draft.derive_ruleset();
            let ruleset_team = draft.select_ruleset(ruleset);
            match ruleset_team.choose_roster(roster) {
                Ok(t)  => t,
                Err(e) => {
                    tracing::warn!("choose_roster initial rejected: {:?}", e);
                    return StatusCode::UNPROCESSABLE_ENTITY.into_response();
                }
            }
        }
        Err(e) => {
            tracing::error!("get_roster_players roster_repo find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(e) = state.team_creation.roster_repository.save(&roster_team, &space_id).await {
        tracing::error!("get_roster_players roster_repo save: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let positions  = build_player_positions(ref_team, ref_repo);
    let cart       = Some(build_cart_vm(&roster_team));
    let staff_rows = build_staff_rows(&roster_team);
    let reroll     = Some(build_reroll_vm(&roster_team));

    RosterPlayersFragment {
        positions,
        team_routes: Default::default(),
        space_id,
        team_id,
        cart,
        staff_rows,
        reroll,
    }.into_response()
}

// ── Fragment ligne joueur ─────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "player-row-fragment.html")]
pub struct PlayerRowFragment {
    pub row:         HiredPlayerRowVm,
    pub team_routes: TeamCreationRoutes,
    pub space_id:    String,
    pub team_id:     String,
    pub cart:        CartVm,
}

impl IntoResponse for PlayerRowFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Handler hire_player ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct HirePlayerBody {
    pub player_id: String,
}

pub async fn hire_player(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
    axum::Json(body):          axum::Json<HirePlayerBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = HirePlayerCommand {
        team_id:   team_id_val,
        space_id:  space_id.clone(),
        player_id: PlayerId(body.player_id.clone()),
    };

    let updated_team = match hire_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Ok(t) => t,
        Err(hire_uc::HirePlayerError::TeamNotFound) =>
            return StatusCode::NOT_FOUND.into_response(),
        Err(hire_uc::HirePlayerError::PlayerNotFound) =>
            return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(hire_uc::HirePlayerError::Domain(e)) =>
            return player_error(hire_uc::domain_error_message(&e)),
        Err(hire_uc::HirePlayerError::Repository(e)) => {
            tracing::error!("hire_player repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let ref_repo = state.references.repository.as_ref();
    let rows     = build_hired_rows(&updated_team, ref_repo);
    let row      = match rows.into_iter().find(|r| r.uid == body.player_id) {
        Some(r) => r,
        None    => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let cart = build_cart_vm(&updated_team);

    PlayerRowFragment {
        row,
        team_routes: Default::default(),
        space_id,
        team_id,
        cart,
    }.into_response()
}

// ── Handler fire_player ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FirePlayerBody {
    pub player_id: String,
}

pub async fn fire_player(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
    axum::Json(body):          axum::Json<FirePlayerBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = FirePlayerCommand {
        team_id:   team_id_val,
        space_id:  space_id.clone(),
        player_id: PlayerId(body.player_id.clone()),
    };

    let updated_team = match fire_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Ok(t) => t,
        Err(fire_uc::FirePlayerError::TeamNotFound) =>
            return StatusCode::NOT_FOUND.into_response(),
        Err(fire_uc::FirePlayerError::PlayerNotFound) =>
            return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(fire_uc::FirePlayerError::Domain(e)) =>
            return player_error(fire_uc::domain_error_message(&e)),
        Err(fire_uc::FirePlayerError::Repository(e)) => {
            tracing::error!("fire_player repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let ref_repo = state.references.repository.as_ref();
    let rows     = build_hired_rows(&updated_team, ref_repo);
    let row      = match rows.into_iter().find(|r| r.uid == body.player_id) {
        Some(r) => r,
        None    => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let cart = build_cart_vm(&updated_team);

    PlayerRowFragment {
        row,
        team_routes: Default::default(),
        space_id,
        team_id,
        cart,
    }.into_response()
}

// ── Fragment ligne staff ──────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "staff-row-fragment.html")]
pub struct StaffRowFragment {
    pub row:         StaffRowVm,
    pub team_routes: TeamCreationRoutes,
    pub space_id:    String,
    pub team_id:     String,
    pub cart:        CartVm,
}

impl IntoResponse for StaffRowFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Fragment ligne relance ────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "reroll-row-fragment.html")]
pub struct RerollRowFragment {
    pub reroll:      RerollVm,
    pub team_routes: TeamCreationRoutes,
    pub space_id:    String,
    pub team_id:     String,
    pub cart:        CartVm,
}

impl IntoResponse for RerollRowFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Handler buy_staff ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StaffBody {
    pub staff_id: String,
}

pub async fn buy_staff(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
    axum::Json(body):          axum::Json<StaffBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = BuyStaffCommand {
        team_id:  team_id_val,
        space_id: space_id.clone(),
        staff_id: StaffId(body.staff_id.clone()),
    };

    let updated_team = match buy_staff_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Ok(t) => t,
        Err(buy_staff_uc::BuyStaffError::TeamNotFound) =>
            return StatusCode::NOT_FOUND.into_response(),
        Err(buy_staff_uc::BuyStaffError::StaffNotFoundInRoster) =>
            return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(buy_staff_uc::BuyStaffError::Domain(ref errors)) =>
            return staff_error(buy_staff_uc::domain_error_message(errors)),
        Err(buy_staff_uc::BuyStaffError::Repository(e)) => {
            tracing::error!("buy_staff repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let staff_rows = build_staff_rows(&updated_team);
    let row = match staff_rows.into_iter().find(|r| r.id == body.staff_id) {
        Some(r) => r,
        None    => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let cart = build_cart_vm(&updated_team);

    StaffRowFragment { row, team_routes: Default::default(), space_id, team_id, cart }.into_response()
}

// ── Handler remove_staff ──────────────────────────────────────────────────────

pub async fn remove_staff(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
    axum::Json(body):          axum::Json<StaffBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = RemoveStaffCommand {
        team_id:  team_id_val,
        space_id: space_id.clone(),
        staff_id: StaffId(body.staff_id.clone()),
    };

    let updated_team = match remove_staff_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Ok(t) => t,
        Err(remove_staff_uc::RemoveStaffError::TeamNotFound) =>
            return StatusCode::NOT_FOUND.into_response(),
        Err(remove_staff_uc::RemoveStaffError::StaffNotFoundInRoster) =>
            return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(remove_staff_uc::RemoveStaffError::Domain(ref e)) =>
            return staff_error(remove_staff_uc::domain_error_message(e)),
        Err(remove_staff_uc::RemoveStaffError::Repository(e)) => {
            tracing::error!("remove_staff repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let staff_rows = build_staff_rows(&updated_team);
    let row = match staff_rows.into_iter().find(|r| r.id == body.staff_id) {
        Some(r) => r,
        None    => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let cart = build_cart_vm(&updated_team);

    StaffRowFragment { row, team_routes: Default::default(), space_id, team_id, cart }.into_response()
}

// ── Handler buy_reroll ────────────────────────────────────────────────────────

pub async fn buy_reroll(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = BuyRerollCommand { team_id: team_id_val, space_id: space_id.clone() };

    let updated_team = match buy_reroll_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Ok(t) => t,
        Err(buy_reroll_uc::BuyRerollError::TeamNotFound) =>
            return StatusCode::NOT_FOUND.into_response(),
        Err(buy_reroll_uc::BuyRerollError::Domain(ref errors)) =>
            return staff_error(buy_reroll_uc::domain_error_message(errors)),
        Err(buy_reroll_uc::BuyRerollError::Repository(e)) => {
            tracing::error!("buy_reroll repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let reroll = build_reroll_vm(&updated_team);
    let cart   = build_cart_vm(&updated_team);

    RerollRowFragment { reroll, team_routes: Default::default(), space_id, team_id, cart }.into_response()
}

// ── Handler remove_reroll ─────────────────────────────────────────────────────

pub async fn remove_reroll(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = RemoveRerollCommand { team_id: team_id_val, space_id: space_id.clone() };

    let updated_team = match remove_reroll_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Ok(t) => t,
        Err(remove_reroll_uc::RemoveRerollError::TeamNotFound) =>
            return StatusCode::NOT_FOUND.into_response(),
        Err(remove_reroll_uc::RemoveRerollError::Repository(e)) => {
            tracing::error!("remove_reroll repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let reroll = build_reroll_vm(&updated_team);
    let cart   = build_cart_vm(&updated_team);

    RerollRowFragment { reroll, team_routes: Default::default(), space_id, team_id, cart }.into_response()
}

// ── Handler submit_team ───────────────────────────────────────────────────────

pub async fn submit_team(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state):              State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = SubmitTeamCommand { team_id: team_id_val };

    match submit_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Ok(()) => {}
        Err(submit_uc::SubmitTeamError::TeamNotFound) =>
            return StatusCode::NOT_FOUND.into_response(),
        Err(submit_uc::SubmitTeamError::Domain(ref errors)) => {
            let msgs: String = errors.iter()
                .map(|e| format!(r#"<p class="table-error">{}</p>"#, submit_uc::domain_error_message(e)))
                .collect();
            return Response::builder()
                .header("HX-Retarget", "#submit-error")
                .header("HX-Reswap", "innerHTML")
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(msgs))
                .unwrap();
        }
        Err(submit_uc::SubmitTeamError::Repository(e)) => {
            tracing::error!("submit_team repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let team_routes: TeamCreationRoutes = Default::default();
    Response::builder()
        .header("HX-Redirect", team_routes.my_teams(&space_id))
        .header("HX-Trigger", r#"{"showToast":"Équipe soumise avec succès !"}"#)
        .body(Body::empty())
        .unwrap()
}
