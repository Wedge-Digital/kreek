use crate::app::auth::auth_backend::AuthSession;
use crate::app::routes::AppRoutes;
use crate::app::teams::ports::MyTeamRow;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct TeamCardVm {
    pub team_id: String,
    pub initials: String,
    pub team_name: String,
    pub roster_name: String,
    pub logo: Option<String>,
    pub status_class: String,
    pub status_label: String,
    pub link: String,
}

pub struct ArchivedTeamCardVm {
    pub team_id: String,
    pub initials: String,
    pub team_name: String,
    pub roster_name: String,
    pub status_label: String,
    pub link: String,
}

#[derive(Template)]
#[template(path = "widgets/my-teams-widget.html")]
pub struct MyTeamsWidgetTemplate {
    pub active_teams: Vec<TeamCardVm>,
    pub archived_teams: Vec<ArchivedTeamCardVm>,
}

impl IntoResponse for MyTeamsWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("my teams widget render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

/// Duplication assumée de `team_detail.rs::status_display` : ici sur des
/// strings issues de `team_proj`, pas sur l'agrégat — à garder synchronisées
/// manuellement si le vocabulaire de statut évolue.
fn status_label_and_class(status: &str, game_phase: Option<&str>) -> (String, String) {
    match status {
        "Rejected" => ("Inscription refusée".into(), "dismissed".into()),
        "Dismissed" => ("Renvoyée".into(), "dismissed".into()),
        "PendingEnrollment" => ("En attente d'inscription".into(), "pending".into()),
        _ => match game_phase {
            Some("ReadyToPlay") => ("Prête à jouer".into(), "ready".into()),
            Some("MatchReporting") => ("Rapport en cours".into(), "phase".into()),
            Some("PlayerImprovement") => ("Phase d'amélioration".into(), "phase".into()),
            Some("Recruitment") => ("Phase de recrutement".into(), "phase".into()),
            Some("Dismissals") => ("Phase de renvois".into(), "phase".into()),
            Some("TemporaryRetirement") => ("Retraite temporaire".into(), "phase".into()),
            Some("OffSeason") => ("Off-season".into(), "offseason".into()),
            _ => ("Inscrite".into(), "ready".into()),
        },
    }
}

/// Classe une ligne dans le groupe actif ou archivé — "archivée" = statut
/// terminal (Rejected/Dismissed), cf. docs/specs/my-teams/mes-equipes/06-domaine.md.
fn classify(
    row: MyTeamRow,
    app_routes: &AppRoutes,
    space_id: &str,
) -> (Option<TeamCardVm>, Option<ArchivedTeamCardVm>) {
    let link = app_routes.teams.team_detail(space_id, &row.team_id);
    let (status_label, status_class) =
        status_label_and_class(&row.status, row.game_phase.as_deref());
    let card_initials = initials(&row.team_name);

    if row.status == "Rejected" || row.status == "Dismissed" {
        let vm = ArchivedTeamCardVm {
            team_id: row.team_id,
            initials: card_initials,
            team_name: row.team_name,
            roster_name: row.roster_name,
            status_label,
            link,
        };
        (None, Some(vm))
    } else {
        let vm = TeamCardVm {
            team_id: row.team_id,
            initials: card_initials,
            team_name: row.team_name,
            roster_name: row.roster_name,
            logo: row.logo_url,
            status_class,
            status_label,
            link,
        };
        (Some(vm), None)
    }
}

fn split_rows(
    rows: Vec<MyTeamRow>,
    app_routes: &AppRoutes,
    space_id: &str,
) -> (Vec<TeamCardVm>, Vec<ArchivedTeamCardVm>) {
    let mut active = Vec::new();
    let mut archived = Vec::new();
    for row in rows {
        match classify(row, app_routes, space_id) {
            (Some(vm), _) => active.push(vm),
            (_, Some(vm)) => archived.push(vm),
            _ => {}
        }
    }
    (active, archived)
}

pub async fn my_teams_widget(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let rows = match state
        .teams
        .team_repository
        .find_by_coach_and_space(&user.id.to_string(), &space_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("my_teams_widget: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let app_routes = AppRoutes::default();
    let (active_teams, archived_teams) = split_rows(rows, &app_routes, &space_id);

    MyTeamsWidgetTemplate {
        active_teams,
        archived_teams,
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_label_and_class_couvre_le_tableau_valide() {
        assert_eq!(
            status_label_and_class("PendingEnrollment", None),
            ("En attente d'inscription".into(), "pending".into())
        );
        assert_eq!(
            status_label_and_class("Enrolled", Some("ReadyToPlay")),
            ("Prête à jouer".into(), "ready".into())
        );
        assert_eq!(
            status_label_and_class("Enrolled", Some("MatchReporting")),
            ("Rapport en cours".into(), "phase".into())
        );
        assert_eq!(
            status_label_and_class("Enrolled", Some("PlayerImprovement")),
            ("Phase d'amélioration".into(), "phase".into())
        );
        assert_eq!(
            status_label_and_class("Enrolled", Some("Recruitment")),
            ("Phase de recrutement".into(), "phase".into())
        );
        assert_eq!(
            status_label_and_class("Enrolled", Some("Dismissals")),
            ("Phase de renvois".into(), "phase".into())
        );
        assert_eq!(
            status_label_and_class("Enrolled", Some("TemporaryRetirement")),
            ("Retraite temporaire".into(), "phase".into())
        );
        assert_eq!(
            status_label_and_class("Enrolled", Some("OffSeason")),
            ("Off-season".into(), "offseason".into())
        );
        assert_eq!(
            status_label_and_class("Enrolled", None),
            ("Inscrite".into(), "ready".into())
        );
        assert_eq!(
            status_label_and_class("Rejected", None),
            ("Inscription refusée".into(), "dismissed".into())
        );
        assert_eq!(
            status_label_and_class("Dismissed", None),
            ("Renvoyée".into(), "dismissed".into())
        );
    }

    #[test]
    fn classify_repartit_actives_et_archivees() {
        let app_routes = AppRoutes::default();
        let row_active = MyTeamRow {
            team_id: "team-a".into(),
            team_name: "Les Korrigans FC".into(),
            roster_name: "Elfes Sylvestres".into(),
            logo_url: None,
            status: "Enrolled".into(),
            game_phase: Some("ReadyToPlay".into()),
        };
        let (active, archived) = classify(row_active, &app_routes, "space-1");
        assert!(active.is_some());
        assert!(archived.is_none());

        let row_archived = MyTeamRow {
            team_id: "team-b".into(),
            team_name: "Chaos Reavers".into(),
            roster_name: "Chaos".into(),
            logo_url: None,
            status: "Rejected".into(),
            game_phase: None,
        };
        let (active, archived) = classify(row_archived, &app_routes, "space-1");
        assert!(active.is_none());
        assert!(archived.is_some());
    }
}
