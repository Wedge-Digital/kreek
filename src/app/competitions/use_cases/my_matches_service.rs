//! La page « Mes matchs » (carte 677, I7-E1-KR-T3).
//!
//! Lecture seule. Elle assemble ce que trois sources savent, chacune par son
//! propre chemin :
//!
//! - les matchs à jouer du coach — la règle de la carte 675, réutilisée telle
//!   quelle (`coach_matches_service`) ;
//! - le bouton de chaque **prochain** match — décidé par `match_report`, lu
//!   par le port `IMatchEntryPort` (carte 676) ;
//! - le nom des compétitions et la règle du hors calendrier — ce BC.
//!
//! Le bouton n'est demandé que pour les prochains matchs : ce sont les seuls
//! qui le portent (I7-B2). Le rapprochement se fait par `pairing_id`, l'ordre
//! de la réponse du port n'étant pas celui de la demande.

use crate::app::auth::domain::user::User;
use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, PairingDisplayDto,
};
use crate::app::competitions::domain::match_entry::MatchEntryKind;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::competitions::ports::{IMatchEntryPort, ITeamInfoPort, MatchEntryLinkDto};
use crate::app::competitions::use_cases::coach_matches_service::{self, CoachMatch};
use crate::app::shared_kernel::identity::ids::SpaceId;
use std::collections::HashMap;

/// Les ports que la page lit — et seulement eux, pour qu'une doublure suffise.
pub struct MyMatchesDeps<'a> {
    pub team_info: &'a dyn ITeamInfoPort,
    pub match_days: &'a dyn IMatchDayRepository,
    pub competitions: &'a dyn ICompetitionRepository,
    pub seasons: &'a dyn ISeasonRepository,
    pub match_entry: &'a dyn IMatchEntryPort,
}

#[derive(Debug)]
pub struct MyMatchesError(pub String);

impl std::fmt::Display for MyMatchesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mes matchs : {}", self.0)
    }
}

/// Le bouton d'un match et où il mène.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchEntry {
    pub kind: MatchEntryKind,
    pub url: String,
}

/// Un match à jouer, tel que la page l'affiche.
pub struct MyMatch {
    pub pairing: PairingDisplayDto,
    /// `None` si la compétition n'est pas retrouvée dans l'espace : on n'en
    /// invente pas le nom.
    pub competition_name: Option<String>,
    pub is_next: bool,
    /// Seul un prochain match en porte un, et seulement si `match_report` en
    /// propose un.
    pub entry: Option<MatchEntry>,
}

pub struct MyMatches {
    pub matches: Vec<MyMatch>,
    /// « Nouveau match » : faux dès qu'une compétition de l'espace interdit
    /// les matchs hors calendrier.
    pub off_schedule_allowed: bool,
}

// arch:no-instrument — service de lecture : assemble la page « Mes matchs », sans intention métier
pub async fn find(
    deps: &MyMatchesDeps<'_>,
    user: &User,
    space_id: &SpaceId,
) -> Result<MyMatches, MyMatchesError> {
    let sid = space_id.to_string();
    let coach_id = user.id.to_string();
    let matches = coach_matches_service::find(deps.team_info, deps.match_days, &coach_id, &sid)
        .await
        .map_err(|e| MyMatchesError(e.to_string()))?;
    let links = entry_links(deps.match_entry, user, &sid, &matches).await?;
    let names = competition_names(deps.competitions, space_id).await?;
    let off_schedule_allowed = off_schedule_allowed(deps.seasons, &sid).await?;
    Ok(MyMatches {
        matches: assemble(matches, links, &names),
        off_schedule_allowed,
    })
}

/// Les boutons des prochains matchs. Aucun prochain match : le port n'est pas
/// consulté.
async fn entry_links(
    port: &dyn IMatchEntryPort,
    user: &User,
    space_id: &str,
    matches: &[CoachMatch],
) -> Result<Vec<MatchEntryLinkDto>, MyMatchesError> {
    let next = next_pairing_ids(matches);
    if next.is_empty() {
        return Ok(vec![]);
    }
    port.find_entry_links(user, space_id, &next)
        .await
        .map_err(MyMatchesError)
}

fn next_pairing_ids(matches: &[CoachMatch]) -> Vec<String> {
    matches
        .iter()
        .filter(|m| m.is_next)
        .map(|m| m.pairing.pairing_id.clone())
        .collect()
}

/// Le nom de chaque compétition de l'espace, par identifiant.
async fn competition_names(
    repo: &dyn ICompetitionRepository,
    space_id: &SpaceId,
) -> Result<HashMap<String, String>, MyMatchesError> {
    let all = repo
        .find_by_space_id(space_id)
        .await
        .map_err(|e| MyMatchesError(e.to_string()))?;
    Ok(all.into_iter().map(|c| (c.id, c.name)).collect())
}

/// « Nouveau match » est permis tant qu'aucune compétition de l'espace
/// n'interdit le hors calendrier — la règle de `ISeasonRepository`.
async fn off_schedule_allowed(
    seasons: &dyn ISeasonRepository,
    space_id: &str,
) -> Result<bool, MyMatchesError> {
    let forbidden = seasons
        .espace_interdit_hors_calendrier(space_id)
        .await
        .map_err(|e| MyMatchesError(e.to_string()))?;
    Ok(!forbidden)
}

/// Rend à chaque match sa compétition et, s'il est le prochain, son bouton.
fn assemble(
    matches: Vec<CoachMatch>,
    links: Vec<MatchEntryLinkDto>,
    names: &HashMap<String, String>,
) -> Vec<MyMatch> {
    let mut links: HashMap<String, MatchEntry> = links
        .into_iter()
        .map(|l| {
            (
                l.pairing_id,
                MatchEntry {
                    kind: l.kind,
                    url: l.url,
                },
            )
        })
        .collect();
    matches
        .into_iter()
        .map(|m| my_match(m, &mut links, names))
        .collect()
}

fn my_match(
    found: CoachMatch,
    links: &mut HashMap<String, MatchEntry>,
    names: &HashMap<String, String>,
) -> MyMatch {
    let entry = match found.is_next {
        true => links.remove(&found.pairing.pairing_id),
        false => None,
    };
    MyMatch {
        competition_name: names.get(&found.competition_id).cloned(),
        pairing: found.pairing,
        is_next: found.is_next,
        entry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::competition::Competition;
    use crate::app::competitions::domain::competition_repository_port::{
        CompetitionBaseInfo, CompetitionRepositoryError, CompetitionSummary, CompetitionWithSeasons,
    };
    use crate::app::competitions::io::repository::tests::fake_season_repository::FakeSeasonRepository;
    use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
    use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::email::Email;
    use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, UserId};
    use async_trait::async_trait;
    use std::sync::Mutex;

    const SPACE: &str = "00000000000000000000000061";
    const COACH: &str = "00000000000000000000000062";

    fn user() -> User {
        User::new(
            UserId::try_new(COACH).unwrap(),
            CoachName::try_new("Testeur".to_string()).unwrap(),
            None,
            Email::try_new("testeur@example.com".to_string()).unwrap(),
            String::new(),
        )
    }

    fn pairing(id: &str) -> PairingDisplayDto {
        PairingDisplayDto {
            pairing_id: id.into(),
            round_id: "r".into(),
            round_name: "Journée 4".into(),
            round_position: 4,
            round_date_start: None,
            round_date_end: None,
            round_day_type: "fixed_date".into(),
            home_team_id: "A".into(),
            home_team_name: "A".into(),
            home_roster_name: "R".into(),
            home_coach_name: String::new(),
            home_logo_url: None,
            home_initials: "A".into(),
            away_team_id: "B".into(),
            away_team_name: "B".into(),
            away_roster_name: "R".into(),
            away_coach_name: String::new(),
            away_logo_url: None,
            away_initials: "B".into(),
            match_status: "upcoming".into(),
            home_score: None,
            away_score: None,
            home_casualties: None,
            away_casualties: None,
            match_report_url: None,
        }
    }

    fn coach_match(id: &str, competition_id: &str, is_next: bool) -> CoachMatch {
        CoachMatch {
            pairing: pairing(id),
            competition_id: competition_id.into(),
            is_next,
        }
    }

    fn link(id: &str, kind: MatchEntryKind) -> MatchEntryLinkDto {
        MatchEntryLinkDto {
            pairing_id: id.into(),
            kind,
            url: format!("/url/{id}"),
        }
    }

    /// Rend les boutons qu'on lui confie, et retient ce qu'on lui a demandé.
    #[derive(Default)]
    struct FakeEntries {
        links: Vec<MatchEntryLinkDto>,
        asked: Mutex<Option<Vec<String>>>,
    }

    #[async_trait]
    impl IMatchEntryPort for FakeEntries {
        async fn find_entry_links(
            &self,
            _: &User,
            _: &str,
            pairing_ids: &[String],
        ) -> Result<Vec<MatchEntryLinkDto>, String> {
            *self.asked.lock().unwrap() = Some(pairing_ids.to_vec());
            Ok(self.links.clone())
        }
    }

    /// Les compétitions de l'espace, par (identifiant, nom).
    struct NamedCompetitions(Vec<(&'static str, &'static str)>);

    #[async_trait]
    impl ICompetitionRepository for NamedCompetitions {
        async fn name_exists_in_space(
            &self,
            _: &CompetitionName,
            _: &SpaceId,
        ) -> Result<bool, CompetitionRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn save(&self, _: &Competition) -> Result<(), CompetitionRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn find_by_space_id(
            &self,
            _: &SpaceId,
        ) -> Result<Vec<CompetitionSummary>, CompetitionRepositoryError> {
            Ok(self.0.iter().map(|(id, name)| summary(id, name)).collect())
        }
        async fn find_with_seasons(
            &self,
            _: &SpaceId,
        ) -> Result<Vec<CompetitionWithSeasons>, CompetitionRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn find_space_id(
            &self,
            _: &CompetitionId,
        ) -> Result<Option<String>, CompetitionRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn find_base_info(
            &self,
            _: &CompetitionId,
        ) -> Result<Option<CompetitionBaseInfo>, CompetitionRepositoryError> {
            unimplemented!("hors du service")
        }
        async fn update_base_info(
            &self,
            _: &CompetitionId,
            _: &CompetitionName,
            _: &CloudinaryImage,
            _: &[CoachId],
        ) -> Result<(), CompetitionRepositoryError> {
            unimplemented!("hors du service")
        }
    }

    fn summary(id: &str, name: &str) -> CompetitionSummary {
        CompetitionSummary {
            id: id.into(),
            name: name.into(),
            logo: String::new(),
            season_id: None,
            status: None,
            season_count: 1,
        }
    }

    #[test]
    fn only_next_matches_are_asked_for_a_button() {
        let matches = vec![
            coach_match("p1", "c", true),
            coach_match("p2", "c", false),
            coach_match("p3", "c", true),
        ];
        assert_eq!(next_pairing_ids(&matches), vec!["p1", "p3"]);
    }

    #[tokio::test]
    async fn entry_links_ask_the_port_for_next_matches_only() {
        let port = FakeEntries {
            links: vec![link("p1", MatchEntryKind::Start)],
            ..Default::default()
        };
        let matches = vec![coach_match("p1", "c", true), coach_match("p2", "c", false)];
        let found = entry_links(&port, &user(), SPACE, &matches).await.unwrap();
        assert_eq!(found, vec![link("p1", MatchEntryKind::Start)]);
        assert_eq!(*port.asked.lock().unwrap(), Some(vec!["p1".to_string()]));
    }

    #[tokio::test]
    async fn without_next_match_the_port_is_not_consulted() {
        let port = FakeEntries::default();
        let matches = vec![coach_match("p2", "c", false)];
        let found = entry_links(&port, &user(), SPACE, &matches).await.unwrap();
        assert!(found.is_empty());
        assert_eq!(*port.asked.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn competition_names_are_keyed_by_id() {
        let repo = NamedCompetitions(vec![("c1", "Ligue de Condate"), ("c2", "Coupe")]);
        let space = SpaceId::try_new(SPACE).unwrap();
        let names = competition_names(&repo, &space).await.unwrap();
        assert_eq!(
            names.get("c1").map(String::as_str),
            Some("Ligue de Condate")
        );
        assert_eq!(names.get("c2").map(String::as_str), Some("Coupe"));
    }

    #[tokio::test]
    async fn a_new_match_is_allowed_when_no_competition_forbids_it() {
        // La doublure partagée ne fait rien interdire : le bouton est permis.
        let allowed = off_schedule_allowed(&FakeSeasonRepository, SPACE).await;
        assert!(allowed.unwrap());
    }

    #[test]
    fn the_button_goes_to_its_next_match_by_pairing_id() {
        let matches = vec![coach_match("p1", "c1", true), coach_match("p2", "c1", true)];
        // L'ordre de la réponse n'est pas celui de la demande.
        let links = vec![
            link("p2", MatchEntryKind::Follow),
            link("p1", MatchEntryKind::Continue),
        ];
        let found = assemble(matches, links, &HashMap::new());
        assert_eq!(
            found[0].entry.as_ref().unwrap().kind,
            MatchEntryKind::Continue
        );
        assert_eq!(found[0].entry.as_ref().unwrap().url, "/url/p1");
        assert_eq!(
            found[1].entry.as_ref().unwrap().kind,
            MatchEntryKind::Follow
        );
    }

    #[test]
    fn a_match_that_is_not_next_never_carries_a_button() {
        let matches = vec![coach_match("p1", "c1", false)];
        let found = assemble(
            matches,
            vec![link("p1", MatchEntryKind::Start)],
            &HashMap::new(),
        );
        assert_eq!(found[0].entry, None);
    }

    #[test]
    fn a_next_match_without_report_has_no_button() {
        let found = assemble(vec![coach_match("p1", "c1", true)], vec![], &HashMap::new());
        assert!(found[0].is_next);
        assert_eq!(found[0].entry, None);
    }

    #[test]
    fn each_match_gets_its_competition_name_or_none() {
        let names = HashMap::from([("c1".to_string(), "Ligue".to_string())]);
        let matches = vec![
            coach_match("p1", "c1", false),
            coach_match("p2", "c9", false),
        ];
        let found = assemble(matches, vec![], &names);
        assert_eq!(found[0].competition_name.as_deref(), Some("Ligue"));
        assert_eq!(found[1].competition_name, None);
    }
}
