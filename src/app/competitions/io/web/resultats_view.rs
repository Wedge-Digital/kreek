use crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto;
use crate::app::shared_kernel::authorization::SpaceProfile;
use crate::app::shared_kernel::common_types::{CompetitionId, SpaceId};
use crate::app::shared_kernel::user::User;
use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::collections::{BTreeMap, HashSet};

pub struct MatchResultatVm {
    pub home_name: String,
    pub home_roster: String,
    pub home_coach: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_roster: String,
    pub away_coach: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
    pub is_in_progress: bool,
    pub is_completed: bool,
    pub home_score: Option<u32>,
    pub away_score: Option<u32>,
    pub home_cas: Option<u32>,
    pub away_cas: Option<u32>,
    pub report_url: Option<String>,
}

pub struct JourneeResultatsVm {
    pub label: String,
    pub matches: Vec<MatchResultatVm>,
}

/// Autorisation à cliquer sur une ligne de résultat pour naviguer vers le
/// rapport de match. `is_admin` (admin d'espace ou de compétition) autorise
/// toutes les lignes ; sinon seules les lignes où l'utilisateur coache l'une
/// des deux équipes sont autorisées.
pub struct ResultAuthorization {
    pub is_admin: bool,
    pub my_team_ids: HashSet<String>,
}

impl ResultAuthorization {
    /// Utilisée par l'onglet admin — l'accès à la page admin est déjà
    /// conditionné à être admin d'espace ou de compétition en amont
    /// (`render_admin_page`), donc toutes les lignes y sont autorisées.
    pub fn unrestricted() -> Self {
        Self {
            is_admin: true,
            my_team_ids: HashSet::new(),
        }
    }

    pub fn allows(&self, home_team_id: &str, away_team_id: &str) -> bool {
        self.is_admin
            || self.my_team_ids.contains(home_team_id)
            || self.my_team_ids.contains(away_team_id)
    }
}

/// Calcule l'autorisation de l'utilisateur courant pour la page publique des
/// résultats : admin d'espace, admin de compétition (par id ou par nom,
/// même règle que `admin_page.rs`), ou coach d'une des équipes inscrites
/// cette saison.
pub async fn compute_authorization(
    state: &AppState,
    user: &User,
    space_id: &SpaceId,
    competition_id: &CompetitionId,
    season_id: &str,
) -> ResultAuthorization {
    let is_space_admin = matches!(
        state
            .spaces
            .space_repository
            .find_member_profile(&user.id, space_id)
            .await,
        Ok(Some(SpaceProfile::SpaceAdmin))
    );

    let is_comp_admin = match state
        .competitions
        .competition_repository
        .find_base_info(competition_id)
        .await
    {
        Ok(Some(info)) => {
            let user_id_str = user.id.to_string();
            let coach_name_str = user.coach_name.clone().into_inner();
            info.admin_ids.contains(&user_id_str) || info.admin_names.contains(&coach_name_str)
        }
        _ => false,
    };

    let is_admin = is_space_admin || is_comp_admin;

    let my_team_ids = if is_admin {
        HashSet::new()
    } else {
        let user_id_str = user.id.to_string();
        state
            .competitions
            .team_info_port
            .find_enrolled_teams(season_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|t| t.coach_id == user_id_str)
            .map(|t| t.team_id)
            .collect()
    };

    ResultAuthorization {
        is_admin,
        my_team_ids,
    }
}

pub async fn load_resultats(
    state: &AppState,
    season_id: &str,
    cursor: Option<i32>,
) -> Result<Vec<PairingDisplayDto>, Response> {
    state
        .competitions
        .match_day_repository
        .list_resultats(season_id, cursor, 3)
        .await
        .map_err(|e| {
            tracing::error!("resultats_view: list_resultats: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}

pub fn build_journees(
    rows: Vec<PairingDisplayDto>,
    max_rounds: usize,
    authz: &ResultAuthorization,
) -> (Vec<JourneeResultatsVm>, Option<i32>) {
    let mut by_round: BTreeMap<i32, (String, Vec<MatchResultatVm>)> = BTreeMap::new();
    for row in rows {
        let entry = by_round
            .entry(row.round_position)
            .or_insert_with(|| (row.round_name.clone(), Vec::new()));
        entry.1.push(to_resultat_vm(row, authz));
    }

    let mut journees: Vec<(i32, JourneeResultatsVm)> = by_round
        .into_iter()
        .map(|(pos, (label, matches))| (pos, JourneeResultatsVm { label, matches }))
        .collect();

    journees.sort_by(|a, b| b.0.cmp(&a.0));
    journees.truncate(max_rounds);

    let next_cursor = if journees.len() == max_rounds {
        journees.last().map(|(pos, _)| *pos)
    } else {
        None
    };

    (journees.into_iter().map(|(_, j)| j).collect(), next_cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_allows_any_match() {
        let authz = ResultAuthorization::unrestricted();
        assert!(authz.allows("any-home", "any-away"));
    }

    #[test]
    fn coach_of_home_team_is_allowed() {
        let authz = ResultAuthorization {
            is_admin: false,
            my_team_ids: HashSet::from(["team-a".to_string()]),
        };
        assert!(authz.allows("team-a", "team-b"));
    }

    #[test]
    fn coach_of_away_team_is_allowed() {
        let authz = ResultAuthorization {
            is_admin: false,
            my_team_ids: HashSet::from(["team-b".to_string()]),
        };
        assert!(authz.allows("team-a", "team-b"));
    }

    #[test]
    fn coach_of_neither_team_is_not_allowed() {
        let authz = ResultAuthorization {
            is_admin: false,
            my_team_ids: HashSet::from(["team-c".to_string()]),
        };
        assert!(!authz.allows("team-a", "team-b"));
    }
}

fn to_resultat_vm(row: PairingDisplayDto, authz: &ResultAuthorization) -> MatchResultatVm {
    let is_completed = row.match_status == "completed";
    let is_in_progress = row.match_status == "in_progress";
    let report_url = if authz.allows(&row.home_team_id, &row.away_team_id) {
        row.match_report_url
    } else {
        None
    };
    MatchResultatVm {
        home_name: row.home_team_name,
        home_roster: row.home_roster_name,
        home_coach: row.home_coach_name,
        home_logo: row.home_logo_url,
        home_initials: row.home_initials,
        away_name: row.away_team_name,
        away_roster: row.away_roster_name,
        away_coach: row.away_coach_name,
        away_logo: row.away_logo_url,
        away_initials: row.away_initials,
        is_in_progress,
        is_completed,
        home_score: row.home_score.map(|v| v as u32),
        away_score: row.away_score.map(|v| v as u32),
        home_cas: row.home_casualties.map(|v| v as u32),
        away_cas: row.away_casualties.map(|v| v as u32),
        report_url,
    }
}
