//! Les matchs à jouer d'un coach, pour la page « Mes matchs » (carte 675).
//!
//! Lecture seule : les équipes inscrites du coach dans l'espace (`teams`, par
//! le port), la saison de chacune, ses rencontres, puis la règle du domaine
//! (`coach_matches`). La saison est lue de l'inscription, jamais d'une URL.
//!
//! **Une équipe qu'on ne sait pas lire est écartée, et journalisée** — sans
//! inscription lisible, ou dont une rencontre est illisible. Deviner son
//! prochain match en sautant la ligne illisible désignerait celui d'après,
//! comme le refuse `next_opponent_service`. Les autres équipes du coach
//! restent affichées.

use crate::app::competitions::domain::coach_matches::{matches_to_play, MatchToPlay, TeamSchedule};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, PairingDisplayDto,
};
use crate::app::competitions::ports::{ITeamInfoPort, TeamEnrollmentDto};
use crate::app::competitions::use_cases::next_opponent_service::scheduled;
use crate::app::shared_kernel::bloodbowl::team::TeamId;

#[derive(Debug)]
pub enum CoachMatchesError {
    Port(String),
}

impl std::fmt::Display for CoachMatchesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Port(cause) => write!(f, "port : {cause}"),
        }
    }
}

/// Un match à jouer : la rencontre telle que la projection l'affiche, la
/// compétition de l'équipe du coach qui la joue, et s'il est le prochain
/// match d'une de ses équipes.
pub struct CoachMatch {
    pub pairing: PairingDisplayDto,
    pub competition_id: String,
    pub is_next: bool,
}

/// Une équipe du coach dont l'inscription et les rencontres ont été lues.
struct LoadedTeam {
    competition_id: String,
    rows: Vec<PairingDisplayDto>,
    schedule: TeamSchedule,
}

// arch:no-instrument — service de lecture : liste les matchs à jouer d'un coach, sans intention métier
pub async fn find(
    team_info: &dyn ITeamInfoPort,
    match_days: &dyn IMatchDayRepository,
    coach_id: &str,
    space_id: &str,
) -> Result<Vec<CoachMatch>, CoachMatchesError> {
    let teams = team_info
        .find_coach_enrolled_teams(coach_id, space_id)
        .await
        .map_err(CoachMatchesError::Port)?;
    let mut loaded = Vec::with_capacity(teams.len());
    for team in &teams {
        loaded.extend(load_team(team_info, match_days, &team.team_id).await?);
    }
    Ok(assemble(loaded))
}

async fn load_team(
    team_info: &dyn ITeamInfoPort,
    match_days: &dyn IMatchDayRepository,
    team_id: &str,
) -> Result<Option<LoadedTeam>, CoachMatchesError> {
    let Some(enrollment) = enrollment(team_info, team_id).await else {
        return Ok(None);
    };
    let rows = match_days
        .list_team_matches(&enrollment.season_id, team_id)
        .await
        .map_err(|e| CoachMatchesError::Port(e.to_string()))?;
    Ok(loaded_team(team_id, enrollment.competition_id, rows))
}

/// L'inscription de l'équipe, ou `None` journalisé : le port la dit inscrite,
/// mais sans saison on ne peut pas lire ses rencontres.
async fn enrollment(team_info: &dyn ITeamInfoPort, team_id: &str) -> Option<TeamEnrollmentDto> {
    match team_info.find_team_enrollment(team_id).await {
        Ok(Some(enrollment)) => Some(enrollment),
        Ok(None) => {
            tracing::warn!(
                team_id,
                "équipe sans inscription lisible : écartée des matchs à jouer"
            );
            None
        }
        Err(cause) => {
            tracing::error!(team_id, %cause, "inscription illisible : équipe écartée des matchs à jouer");
            None
        }
    }
}

/// Les rencontres lues de l'équipe, ou `None` journalisé si l'une d'elles est
/// illisible — l'équipe entière est alors écartée.
fn loaded_team(
    team_id: &str,
    competition_id: String,
    rows: Vec<PairingDisplayDto>,
) -> Option<LoadedTeam> {
    let team = TeamId::try_new(team_id).ok();
    let pairings = rows.iter().map(scheduled).collect::<Option<Vec<_>>>();
    let (Some(team), Some(pairings)) = (team, pairings) else {
        tracing::error!(
            team_id,
            "rencontre illisible : équipe écartée des matchs à jouer"
        );
        return None;
    };
    Some(LoadedTeam {
        competition_id,
        rows,
        schedule: TeamSchedule { team, pairings },
    })
}

/// Applique la règle du domaine, puis rend à chaque match sa ligne
/// d'affichage et sa compétition.
fn assemble(loaded: Vec<LoadedTeam>) -> Vec<CoachMatch> {
    let mut schedules = Vec::with_capacity(loaded.len());
    let mut rows = Vec::new();
    for team in loaded {
        schedules.push(team.schedule);
        let competition_id = team.competition_id;
        rows.extend(team.rows.into_iter().map(|r| (competition_id.clone(), r)));
    }
    matches_to_play(&schedules)
        .into_iter()
        .filter_map(|m| with_row(m, &mut rows))
        .collect()
}

/// La ligne d'affichage de ce match — la première trouvée : une rencontre
/// entre deux équipes du coach figure dans les deux calendriers.
fn with_row(found: MatchToPlay, rows: &mut Vec<(String, PairingDisplayDto)>) -> Option<CoachMatch> {
    let pairing_id = found.pairing_id.to_string();
    let index = rows.iter().position(|(_, r)| r.pairing_id == pairing_id)?;
    let (competition_id, pairing) = rows.swap_remove(index);
    Some(CoachMatch {
        pairing,
        competition_id,
        is_next: found.is_next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
    use crate::app::competitions::domain::match_day_repository_port::{
        LatestResultDto, MatchDayRepositoryError, NewPairingProjection,
    };
    use crate::app::competitions::ports::{CoachTeamDto, TeamInfoDto};
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId};
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Une rencontre de la projection. Ses identifiants sont tirés une fois :
    /// la même rend la même ligne dans les deux calendriers d'un derby.
    struct Row {
        pairing: PairingId,
        round: MatchId,
        home: TeamId,
        away: TeamId,
        position: i32,
        status: &'static str,
        date_end: Option<&'static str>,
    }

    fn fixture(home: &TeamId, away: &TeamId, position: i32, status: &'static str) -> Row {
        Row {
            pairing: PairingId::new(),
            round: MatchId::new(),
            home: home.clone(),
            away: away.clone(),
            position,
            status,
            date_end: None,
        }
    }

    impl Row {
        fn dated(self, date_end: &'static str) -> Self {
            Self {
                date_end: Some(date_end),
                ..self
            }
        }
    }

    fn row(r: &Row) -> PairingDisplayDto {
        PairingDisplayDto {
            pairing_id: r.pairing.to_string(),
            round_id: r.round.to_string(),
            round_name: format!("Journée {}", r.position),
            round_position: r.position,
            round_date_start: None,
            round_date_end: r.date_end.map(String::from),
            round_day_type: "fixed_date".into(),
            home_team_id: r.home.to_string(),
            home_team_name: "A".into(),
            home_roster_name: "R".into(),
            home_coach_name: String::new(),
            home_logo_url: None,
            home_initials: "A".into(),
            away_team_id: r.away.to_string(),
            away_team_name: "B".into(),
            away_roster_name: "R".into(),
            away_coach_name: String::new(),
            away_logo_url: None,
            away_initials: "B".into(),
            match_status: r.status.into(),
            home_score: None,
            away_score: None,
            home_casualties: None,
            away_casualties: None,
            match_report_url: None,
        }
    }

    /// Les équipes du coach, et pour chacune sa saison — absente : pas
    /// d'inscription ; `"err"` : le port échoue.
    struct FakeTeams(Vec<(TeamId, Option<&'static str>)>);

    #[async_trait]
    impl ITeamInfoPort for FakeTeams {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(
            &self,
            team_id: &str,
        ) -> Result<Option<TeamEnrollmentDto>, String> {
            let season = self.0.iter().find(|(t, _)| t.to_string() == team_id);
            match season.and_then(|(_, s)| *s) {
                Some("err") => Err("panne".into()),
                Some(season) => Ok(Some(TeamEnrollmentDto {
                    competition_id: format!("comp-{season}"),
                    season_id: season.into(),
                })),
                None => Ok(None),
            }
        }
        async fn find_coach_enrolled_teams(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<CoachTeamDto>, String> {
            Ok(self
                .0
                .iter()
                .map(|(t, _)| CoachTeamDto {
                    team_id: t.to_string(),
                    team_name: String::new(),
                })
                .collect())
        }
    }

    struct FailingTeams;

    #[async_trait]
    impl ITeamInfoPort for FailingTeams {
        async fn find_enrolled_teams(&self, _: &str) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_names(&self, _: &[String]) -> Result<Vec<TeamInfoDto>, String> {
            Ok(vec![])
        }
        async fn find_team_enrollment(&self, _: &str) -> Result<Option<TeamEnrollmentDto>, String> {
            Ok(None)
        }
        async fn find_coach_enrolled_teams(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<CoachTeamDto>, String> {
            Err("panne".into())
        }
    }

    /// Les rencontres par (saison, équipe), rendues une fois.
    #[derive(Default)]
    struct FakeMatchDays(Mutex<Vec<(String, String, PairingDisplayDto)>>);

    impl FakeMatchDays {
        fn add(&self, season: &str, team: &TeamId, r: &Row) {
            let entry = (season.to_string(), team.to_string(), row(r));
            self.0.lock().unwrap().push(entry);
        }
    }

    #[async_trait]
    impl IMatchDayRepository for FakeMatchDays {
        async fn list_team_matches(
            &self,
            season_id: &str,
            team_id: &str,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            let mut all = self.0.lock().unwrap();
            let (mine, rest): (Vec<_>, Vec<_>) = all
                .drain(..)
                .partition(|(s, t, _)| s == season_id && t == team_id);
            *all = rest;
            Ok(mine.into_iter().map(|(_, _, r)| r).collect())
        }
        async fn find_by_season(&self, _: &str) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn find_by_id(&self, _: &str) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn save_match_day(&self, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn delete_match_day(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn save_pairings(
            &self,
            _: &str,
            _: &[(Pairing, NewPairingProjection)],
        ) -> Result<(), MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn save_pairing(
            &self,
            _: &str,
            _: &Pairing,
            _: &NewPairingProjection,
        ) -> Result<(), MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn find_pairing_id(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Option<String>, MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn delete_pairing(&self, _: &str) -> Result<(), MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn move_pairing(&self, _: &str, _: &MatchDay) -> Result<(), MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn ensure_match_days_from_structure(
            &self,
            _: &str,
            _: &[(String, String, String, Option<String>, Option<String>)],
        ) -> Result<(), MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn list_resultats(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn list_calendrier(
            &self,
            _: &str,
            _: Option<i32>,
            _: u32,
        ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn list_latest_completed_results(
            &self,
            _: &str,
            _: i64,
        ) -> Result<Vec<LatestResultDto>, MatchDayRepositoryError> {
            unimplemented!("hors du service")
        }
    }

    fn pairing_ids(matches: &[CoachMatch]) -> Vec<String> {
        matches
            .iter()
            .map(|m| m.pairing.pairing_id.clone())
            .collect()
    }

    /// Le critère de la carte : deux équipes inscrites, un derby entre elles,
    /// une troisième sans inscription écartée. Les matchs sont triés, sans
    /// doublon, le prochain de chacune marqué.
    #[tokio::test]
    async fn find_lists_the_matches_of_two_enrolled_teams() {
        let (a, b, ghost, x) = (TeamId::new(), TeamId::new(), TeamId::new(), TeamId::new());
        let derby = fixture(&a, &b, 1, "upcoming").dated("2026-12-20");
        let later = fixture(&x, &a, 2, "upcoming").dated("2026-12-01");
        let days = FakeMatchDays::default();
        days.add("s1", &a, &derby);
        days.add("s1", &a, &later);
        days.add("s1", &b, &derby);
        days.add("s1", &b, &fixture(&b, &x, 0, "completed"));
        let teams = vec![(a, Some("s1")), (ghost, None), (b, Some("s1"))];

        let matches = find(&FakeTeams(teams), &days, "coach", "space").await;

        let matches = matches.unwrap();
        let expected = vec![later.pairing.to_string(), derby.pairing.to_string()];
        assert_eq!(pairing_ids(&matches), expected);
        assert!(!matches[0].is_next, "a joue le derby d'abord, journée 1");
        assert!(matches[1].is_next);
        assert!(matches.iter().all(|m| m.competition_id == "comp-s1"));
    }

    #[tokio::test]
    async fn find_fails_when_the_teams_cannot_be_read() {
        let days = FakeMatchDays::default();
        let outcome = find(&FailingTeams, &days, "coach", "space").await;
        assert!(matches!(outcome, Err(CoachMatchesError::Port(_))));
    }

    /// Une équipe dont l'inscription manque ou ne se lit pas est écartée, pas
    /// une erreur de toute la liste.
    #[tokio::test]
    async fn load_team_skips_a_team_without_readable_enrollment() {
        let (none, failing) = (TeamId::new(), TeamId::new());
        let teams = FakeTeams(vec![(none.clone(), None), (failing.clone(), Some("err"))]);
        let days = FakeMatchDays::default();
        let none_id = none.to_string();
        let failing_id = failing.to_string();
        assert!(load_team(&teams, &days, &none_id).await.unwrap().is_none());
        assert!(load_team(&teams, &days, &failing_id)
            .await
            .unwrap()
            .is_none());
        assert!(enrollment(&teams, &none_id).await.is_none());
        assert!(enrollment(&teams, &failing_id).await.is_none());
    }

    #[tokio::test]
    async fn enrollment_returns_the_season_of_an_enrolled_team() {
        let team = TeamId::new();
        let teams = FakeTeams(vec![(team.clone(), Some("s9"))]);
        let found = enrollment(&teams, &team.to_string()).await.unwrap();
        assert_eq!(found.season_id, "s9");
        assert_eq!(found.competition_id, "comp-s9");
    }

    /// Une rencontre illisible écarte l'équipe : sauter la ligne ferait
    /// prendre le match d'après pour le prochain.
    #[test]
    fn loaded_team_rejects_a_team_with_an_unreadable_pairing() {
        let (a, x) = (TeamId::new(), TeamId::new());
        let odd = fixture(&a, &x, 1, "postponed");
        let fine = fixture(&a, &x, 2, "upcoming");
        let rows = vec![row(&odd), row(&fine)];
        assert!(loaded_team(&a.to_string(), "c".into(), rows).is_none());
        let loaded = loaded_team(&a.to_string(), "c".into(), vec![row(&fine)]).unwrap();
        assert_eq!(loaded.schedule.pairings.len(), 1);
        assert!(loaded_team("pas-un-id", "c".into(), vec![]).is_none());
    }

    /// Le derby figure deux fois dans les lignes, une fois dans le résultat.
    #[test]
    fn assemble_gives_each_match_one_row_and_its_competition() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let derby = fixture(&a, &b, 1, "in_progress");
        let loaded = vec![
            loaded_team(&a.to_string(), "c1".into(), vec![row(&derby)]).unwrap(),
            loaded_team(&b.to_string(), "c1".into(), vec![row(&derby)]).unwrap(),
        ];
        let matches = assemble(loaded);
        assert_eq!(pairing_ids(&matches), vec![derby.pairing.to_string()]);
        assert_eq!(matches[0].competition_id, "c1");
        assert!(matches[0].is_next);
    }

    #[test]
    fn with_row_takes_the_row_of_the_match_or_nothing() {
        let line = fixture(&TeamId::new(), &TeamId::new(), 1, "upcoming");
        let mut rows = vec![("c1".to_string(), row(&line))];
        let found = MatchToPlay {
            pairing_id: line.pairing.clone(),
            is_next: true,
        };
        let taken = with_row(found.clone(), &mut rows).unwrap();
        assert_eq!(taken.pairing.pairing_id, line.pairing.to_string());
        assert!(taken.is_next);
        assert!(with_row(found, &mut rows).is_none(), "ligne déjà prise");
    }
}
