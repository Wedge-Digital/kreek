use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::match_report_repository_port::MatchActionRow;
use crate::app::match_report::domain::value_objects::{MatchActionType, TeamSide};
use crate::app::match_report::ports::{ICoachDataPort, IKeywordCatalogPort};
use crate::app::match_report::use_cases::match_report_access_service::{
    can_delete_actions, load_entry_scope, AccesRapportDeps, EntryScope, PorteeRapport,
};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::collections::HashMap;

pub struct ActionRowVm {
    pub action_id: String,
    pub turn: i16,
    pub player_name: String,
    pub player_position: String,
    pub action_label: String,
    /// « Nain » plutôt que `DWARF` : le libellé, résolu au catalogue.
    pub hate_label: Option<String>,
    pub action_icon: String,
    /// `home` ou `away` : le liseré du camp, les deux camps partageant le fil.
    pub side_class: &'static str,
    pub team_name: String,
    /// « toi », le nom de l'autre coach, ou rien quand l'auteur est inconnu —
    /// jamais un auteur inventé.
    pub recorded_by: Option<String>,
    /// `None` pour qui ne peut pas supprimer : le ✕ n'est pas rendu.
    pub delete_url: Option<String>,
}

/// Ce qu'une ligne du journal ne lit pas sur elle-même.
pub struct ActionLogContext<'a> {
    pub space_id: &'a str,
    pub match_report_id: &'a str,
    pub current_user_id: String,
    pub home_team_name: String,
    pub away_team_name: String,
    /// Les noms des auteurs autres que l'utilisateur courant.
    pub recorder_names: HashMap<String, String>,
    /// Le droit vient du service d'accès, jamais du gabarit.
    pub can_delete: bool,
    pub keyword_catalog: &'a dyn IKeywordCatalogPort,
}

impl ActionRowVm {
    pub fn from_row(row: MatchActionRow, ctx: &ActionLogContext<'_>) -> Self {
        let (action_label, action_icon) = parse_action(&row.action_json);
        let (side_class, team_name) = match row.team_side {
            TeamSide::Home => ("home", &ctx.home_team_name),
            TeamSide::Away => ("away", &ctx.away_team_name),
        };
        Self {
            hate_label: libelle_de_haine(&row.action_json, ctx.keyword_catalog),
            recorded_by: recorded_by_label(row.recorded_by.as_deref(), ctx),
            delete_url: delete_url(&row.action_id, ctx),
            side_class,
            team_name: team_name.clone(),
            action_id: row.action_id,
            turn: row.turn_number,
            player_name: row.player_display_name,
            player_position: row.player_position,
            action_label,
            action_icon,
        }
    }
}

/// « toi » pour l'utilisateur courant, sinon le nom résolu par le port.
fn recorded_by_label(recorded_by: Option<&str>, ctx: &ActionLogContext<'_>) -> Option<String> {
    let id = recorded_by?;
    if id == ctx.current_user_id {
        return Some("toi".to_string());
    }
    ctx.recorder_names.get(id).cloned()
}

fn delete_url(action_id: &str, ctx: &ActionLogContext<'_>) -> Option<String> {
    ctx.can_delete.then(|| {
        AppRoutes::default().match_report.delete_action(
            ctx.space_id,
            ctx.match_report_id,
            action_id,
        )
    })
}

#[derive(Template)]
#[template(path = "action-log-widget.html")]
pub struct ActionLogTemplate {
    pub actions: Vec<ActionRowVm>,
}

impl IntoResponse for ActionLogTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Le journal des deux camps, du plus récent au plus ancien (carte 635).
pub async fn get_action_log(
    auth_session: AuthSession,
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match render_action_log(&state, &user, &space_id, &mr_id).await {
        Ok(template) => template.into_response(),
        Err(status) => status.into_response(),
    }
}

async fn render_action_log(
    state: &AppState,
    user: &User,
    space_id: &str,
    mr_id: &str,
) -> Result<ActionLogTemplate, StatusCode> {
    let scope = load_scope(state, mr_id).await?;
    let entry = load_entry(state, mr_id).await?;
    let rows = load_rows(state, mr_id).await?;
    let ctx = log_context(state, user, (space_id, mr_id), (&scope, &entry), &rows).await;
    let actions = rows
        .into_iter()
        .map(|r| ActionRowVm::from_row(r, &ctx))
        .collect();
    Ok(ActionLogTemplate { actions })
}

/// Les noms des équipes et des auteurs, et le droit de supprimer : trois
/// questions indépendantes, posées ensemble.
async fn log_context<'a>(
    state: &'a AppState,
    user: &User,
    (space_id, match_report_id): (&'a str, &'a str),
    (scope, entry): (&PorteeRapport, &EntryScope),
    rows: &[MatchActionRow],
) -> ActionLogContext<'a> {
    let current_user_id = user.id.to_string();
    let deps = AccesRapportDeps::from_state(state);
    let ((home_team_name, away_team_name), recorder_names, can_delete) = tokio::join!(
        team_names(state, scope),
        recorder_names(
            state.match_report.coach_data.as_ref(),
            rows,
            &current_user_id
        ),
        can_delete_actions(&deps, user, space_id, scope, entry),
    );
    ActionLogContext {
        space_id,
        match_report_id,
        current_user_id,
        home_team_name,
        away_team_name,
        recorder_names,
        can_delete,
        keyword_catalog: state.match_report.keyword_catalog.as_ref(),
    }
}

async fn load_scope(state: &AppState, mr_id: &str) -> Result<PorteeRapport, StatusCode> {
    let repo = state.match_report.match_report_repo.as_ref();
    match repo.find_access_scope(mr_id).await {
        Ok(Some((competition_id, home_team_id, away_team_id))) => Ok(PorteeRapport {
            competition_id,
            home_team_id,
            away_team_id,
        }),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("journal : portée de {mr_id} : {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Le mode de saisie et le démarreur, d'où se décide le ✕ (carte 634).
async fn load_entry(state: &AppState, mr_id: &str) -> Result<EntryScope, StatusCode> {
    let repo = state.match_report.match_report_repo.as_ref();
    match load_entry_scope(repo, mr_id).await {
        Ok(Some(entry)) => Ok(entry),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("journal : mode de saisie de {mr_id} : {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn load_rows(state: &AppState, mr_id: &str) -> Result<Vec<MatchActionRow>, StatusCode> {
    let repo = state.match_report.match_report_repo.as_ref();
    repo.find_actions_by_match(mr_id).await.map_err(|e| {
        tracing::error!("journal : actions de {mr_id} : {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// Les noms des deux équipes : `(domicile, visiteur)`. Une équipe introuvable
/// rend un nom vide, que le gabarit n'affiche pas.
async fn team_names(state: &AppState, scope: &PorteeRapport) -> (String, String) {
    let team_data = state.match_report.team_data.as_ref();
    let (home, away) = tokio::join!(
        team_data.find_team_info(&scope.home_team_id),
        team_data.find_team_info(&scope.away_team_id),
    );
    (
        home.map(|t| t.team_name).unwrap_or_default(),
        away.map(|t| t.team_name).unwrap_or_default(),
    )
}

/// Le nom de chaque auteur du journal, l'utilisateur courant excepté : il est
/// « toi ». Un coach que le port ne connaît pas reste sans nom.
async fn recorder_names(
    coach_data: &dyn ICoachDataPort,
    rows: &[MatchActionRow],
    current_user_id: &str,
) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for id in rows.iter().filter_map(|r| r.recorded_by.as_deref()) {
        if id == current_user_id || names.contains_key(id) {
            continue;
        }
        if let Some(name) = coach_data.find_coach_name(id).await {
            names.insert(id.to_string(), name);
        }
    }
    names
}

fn parse_action(json: &serde_json::Value) -> (String, String) {
    if let Ok(action) = serde_json::from_value::<MatchActionType>(json.clone()) {
        return (
            action_type_label(&action),
            action_type_icon(&action).to_string(),
        );
    }
    ("Action inconnue".to_string(), "❓".to_string())
}

/// Le libellé du mot-clef haï, s'il y en a un.
///
/// Un uid que le catalogue ne connaît plus est **affiché tel quel** plutôt
/// qu'escamoté : une Haine qui disparaît de l'écran sans un mot serait le genre
/// de silence que cette série corrige.
fn libelle_de_haine(
    json: &serde_json::Value,
    catalogue: &dyn crate::app::match_report::ports::IKeywordCatalogPort,
) -> Option<String> {
    let MatchActionType::Blesse {
        hatred: Some(mot), ..
    } = serde_json::from_value::<MatchActionType>(json.clone()).ok()?
    else {
        return None;
    };
    let uid = mot.to_string();
    Some(
        catalogue
            .find_hateable(&uid)
            .map(|k| k.label)
            .unwrap_or(uid),
    )
}

fn action_type_label(action: &MatchActionType) -> String {
    match action {
        MatchActionType::Touchdown => "Touchdown".into(),
        MatchActionType::Passe => "Passe".into(),
        MatchActionType::Interception => "Interception".into(),
        MatchActionType::Agression => "Agression".into(),
        MatchActionType::Lancer => "Lancer".into(),
        MatchActionType::Sortie => "Sortie".into(),
        MatchActionType::Mvp => "MVP".into(),
        MatchActionType::Blesse { .. } => "Blessé".into(),
    }
}

fn action_type_icon(action: &MatchActionType) -> &'static str {
    match action {
        MatchActionType::Touchdown => "🏈",
        MatchActionType::Passe => "🎯",
        MatchActionType::Interception => "🛡️",
        MatchActionType::Agression => "💥",
        MatchActionType::Lancer => "🤾",
        MatchActionType::Sortie => "🩸",
        MatchActionType::Mvp => "⭐",
        MatchActionType::Blesse { .. } => "🚑",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::ports::KeywordDto;

    const ME: &str = "coach-courant";
    const YANN: &str = "coach-yann";

    struct NoKeywords;

    impl IKeywordCatalogPort for NoKeywords {
        fn list_hateable(&self) -> Vec<KeywordDto> {
            vec![]
        }
        fn find_hateable(&self, _: &str) -> Option<KeywordDto> {
            None
        }
    }

    /// Connaît Yann, et lui seul.
    struct FakeCoachData;

    #[async_trait::async_trait]
    impl ICoachDataPort for FakeCoachData {
        async fn find_coach_name(&self, coach_id: &str) -> Option<String> {
            (coach_id == YANN).then(|| "Yann".to_string())
        }
    }

    fn row(side: TeamSide, recorded_by: Option<&str>) -> MatchActionRow {
        MatchActionRow {
            action_id: "action".to_string(),
            team_side: side,
            turn_number: 7,
            player_id: "joueur".to_string(),
            player_type: "regular".to_string(),
            action_json: serde_json::to_value(MatchActionType::Touchdown).unwrap(),
            player_display_name: "Borin".to_string(),
            player_position: "Bloqueur".to_string(),
            recorded_by: recorded_by.map(str::to_string),
        }
    }

    fn ctx(can_delete: bool) -> ActionLogContext<'static> {
        ActionLogContext {
            space_id: "espace",
            match_report_id: "rapport",
            current_user_id: ME.to_string(),
            home_team_name: "Nains de Fer".to_string(),
            away_team_name: "London Rats".to_string(),
            recorder_names: HashMap::from([(YANN.to_string(), "Yann".to_string())]),
            can_delete,
            keyword_catalog: &NoKeywords,
        }
    }

    #[test]
    fn the_current_user_is_named_you() {
        let vm = ActionRowVm::from_row(row(TeamSide::Home, Some(ME)), &ctx(true));
        assert_eq!(vm.recorded_by.as_deref(), Some("toi"));
    }

    #[test]
    fn the_other_coach_is_named() {
        let vm = ActionRowVm::from_row(row(TeamSide::Away, Some(YANN)), &ctx(true));
        assert_eq!(vm.recorded_by.as_deref(), Some("Yann"));
    }

    /// Un auteur absent ou inconnu du port ne reçoit pas de nom inventé.
    #[test]
    fn an_unknown_author_stays_unnamed() {
        let none = ActionRowVm::from_row(row(TeamSide::Home, None), &ctx(true));
        let stranger = ActionRowVm::from_row(row(TeamSide::Home, Some("autre")), &ctx(true));
        assert_eq!((none.recorded_by, stranger.recorded_by), (None, None));
    }

    #[test]
    fn each_line_names_its_side_and_team() {
        let home = ActionRowVm::from_row(row(TeamSide::Home, None), &ctx(true));
        let away = ActionRowVm::from_row(row(TeamSide::Away, None), &ctx(true));
        assert_eq!(
            (home.side_class, home.team_name.as_str()),
            ("home", "Nains de Fer")
        );
        assert_eq!(
            (away.side_class, away.team_name.as_str()),
            ("away", "London Rats")
        );
    }

    #[test]
    fn no_cross_for_who_cannot_delete() {
        let vm = ActionRowVm::from_row(row(TeamSide::Home, Some(ME)), &ctx(false));
        assert_eq!(vm.delete_url, None);
    }

    #[test]
    fn a_cross_for_who_can_delete() {
        let vm = ActionRowVm::from_row(row(TeamSide::Home, Some(ME)), &ctx(true));
        assert!(vm
            .delete_url
            .is_some_and(|url| url.ends_with("/actions/action")));
    }

    /// Le gabarit, et non seulement le VM : sans droit, aucun bouton ✕ rendu.
    #[test]
    fn the_rendered_log_has_no_cross_without_the_right() {
        let actions = vec![ActionRowVm::from_row(
            row(TeamSide::Away, Some(YANN)),
            &ctx(false),
        )];
        let html = ActionLogTemplate { actions }.render().unwrap();
        assert!(html.contains("London Rats · saisi par Yann"));
        assert!(!html.contains("mr-log-delete"));
    }

    /// L'utilisateur courant n'est pas demandé au port : il est « toi ».
    #[tokio::test]
    async fn recorder_names_skip_the_current_user() {
        let rows = vec![
            row(TeamSide::Home, Some(ME)),
            row(TeamSide::Away, Some(YANN)),
            row(TeamSide::Away, None),
        ];
        let names = recorder_names(&FakeCoachData, &rows, ME).await;
        assert_eq!(
            names,
            HashMap::from([(YANN.to_string(), "Yann".to_string())])
        );
    }
}
