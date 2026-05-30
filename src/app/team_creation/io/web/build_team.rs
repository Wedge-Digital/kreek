use askama::Template;
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
use crate::app::shared_kernel::common_types::{Entity, EntityId};
use crate::app::shared_kernel::staff::{StaffId, StaffKind, StaffMaxQuantity, StaffName, StaffPrice};
use crate::app::team_creation::domain::roster::{
    CrossLimit, PlayerId, PlayerDefinition, PlayerMaxQuantity, PlayerName,
    PlayerPrice, RerollBasePrice, Roster, RosterId, RosterName,
};
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::domain::team_staff::TeamStaff;
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::app::team_creation::use_cases::commands::HirePlayerCommand;
use crate::app::team_creation::use_cases::hire_player as hire_uc;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;

// ── Roster builder: references → domain ──────────────────────────────────────

fn staff_kind(uid: &str) -> StaffKind {
    match uid {
        "APOTHECARY"      => StaffKind::Apothecary,
        "CHEERLEADERS"    => StaffKind::Cheerleaders,
        "COACH_ASSISTANTS" => StaffKind::CoachAssistant,
        _                 => StaffKind::CoachAssistant,
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
        id:                 RosterId(ref_team.uid.clone()),
        name:               RosterName(ref_team.name.clone()),
        player_definitions,
        allowed_staff,
        cross_limits:       vec![],   // cross_limit is always [] in current data
        reroll_price:       RerollBasePrice(ref_team.reroll_cost / 1000),
    }
}

// ── Hired row builder ─────────────────────────────────────────────────────────

pub fn build_hired_rows(
    team:     &RosterSelectedTeam,
    ref_repo: &dyn IReferenceRepository,
) -> Vec<HiredPlayerRowVm> {
    team.roster.player_definitions.iter().map(|def| {
        let quantity = team.hired_players().iter().filter(|p| p.id == def.id).count();
        let line_cost_kpo = quantity as u32 * def.price.0;
        let is_max = quantity >= def.max_quantity.0 as usize
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

// ── Page complète ─────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "build-team.html")]
pub struct BuildTeamTemplate {
    pub web_routes:  WebRoutes,
    pub team_routes: TeamCreationRoutes,
    pub space_id:    String,
    pub team_id:     String,
    pub rosters:     Vec<RosterPickerItemWithTier>,
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
            tracing::error!("build_team find_by_id {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let rosters = build_roster_items_with_tiers(
        state.references.repository.as_ref(),
        draft.creation_rules(),
    );

    BuildTeamTemplate {
        web_routes:  Default::default(),
        team_routes: Default::default(),
        space_id,
        team_id,
        rosters,
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

    // Créer ou mettre à jour le RosterSelectedTeam en DB
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let roster = build_roster_from_ref(ref_team, ref_repo);

    // Charger le DraftTeam pour obtenir le ruleset
    let draft: DraftTeam = match state.team_creation.team_repository.find_by_id(&team_id_val).await {
        Ok(Some(t)) => t,
        Ok(None)    => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_roster_players draft find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Résoudre le RosterSelectedTeam (créer ou changer de roster)
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
            let ruleset = draft.derive_ruleset();
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

    let positions = build_player_positions(ref_team, ref_repo);

    RosterPlayersFragment {
        positions,
        team_routes: Default::default(),
        space_id,
        team_id,
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
        Err(hire_uc::HirePlayerError::Domain(e)) => {
            let msg = hire_uc::domain_error_message(&e);
            let frag = format!(
                r#"<tr id="player-row-{}"><td colspan="13" class="player-row-error">{}</td></tr>"#,
                body.player_id, msg
            );
            return (StatusCode::UNPROCESSABLE_ENTITY, Html(frag)).into_response();
        }
        Err(hire_uc::HirePlayerError::Repository(e)) => {
            tracing::error!("hire_player repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let ref_repo = state.references.repository.as_ref();
    let rows = build_hired_rows(&updated_team, ref_repo);
    let row = match rows.into_iter().find(|r| r.uid == body.player_id) {
        Some(r) => r,
        None    => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    PlayerRowFragment {
        row,
        team_routes: Default::default(),
        space_id,
        team_id,
    }.into_response()
}