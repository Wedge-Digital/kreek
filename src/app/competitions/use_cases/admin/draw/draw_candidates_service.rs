//! L'étape « Présents » : les équipes que l'organisateur peut cocher, poule par
//! poule, avec leur nombre de matchs joués.
//!
//! Le chargement est celui de tout tirage (`load_draw_context`) : l'écran montre
//! exactement les équipes et les poules que l'aperçu tirera ensuite.

use crate::app::competitions::domain::draw::MatchesPlayed;
use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::ports::TeamInfoDto;
use crate::app::competitions::use_cases::admin::draw::draw_input::{
    build_matches_played, load_draw_context, DrawContext, DrawDeps, DrawLoadError,
};
use crate::app::competitions::use_cases::admin::team_enrollment::filter_enrolled_team_ids;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct DrawCandidates {
    pub groups: Vec<CandidateGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateGroup {
    pub group_name: String,
    pub teams: Vec<Candidate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub matches_played: u32,
}

#[derive(Debug)]
pub enum DrawCandidatesError {
    RoundNotFound,
    /// R9 — journée de repos, ou déjà pourvue de rencontres.
    Refused(DomainError),
    Repository(String),
}

impl From<DrawLoadError> for DrawCandidatesError {
    fn from(e: DrawLoadError) -> Self {
        match e {
            DrawLoadError::RoundNotFound => Self::RoundNotFound,
            DrawLoadError::Repository(msg) => Self::Repository(msg),
        }
    }
}

// arch:no-instrument — service de lecture : assemble l'étape « Présents », sans intention métier
pub async fn load(
    round_id: &str,
    season_id: &str,
    deps: &DrawDeps<'_>,
) -> Result<DrawCandidates, DrawCandidatesError> {
    let ctx = load_draw_context(round_id, season_id, deps).await?;
    candidates_of(&ctx)
}

/// Les candidates d'une journée qui peut recevoir un tirage. Une poule sans
/// inscrite n'apparaît pas : il n'y a rien à y cocher.
pub fn candidates_of(ctx: &DrawContext) -> Result<DrawCandidates, DrawCandidatesError> {
    ctx.match_day
        .can_receive_draw()
        .map_err(DrawCandidatesError::Refused)?;
    let played = build_matches_played(&ctx.season_days, &ctx.match_day.id.to_string());
    let groups = ctx
        .groups
        .iter()
        .map(|group| CandidateGroup {
            group_name: group.group_name.clone(),
            teams: group_candidates(&group.team_ids, ctx, &played),
        })
        .filter(|group| !group.teams.is_empty())
        .collect();
    Ok(DrawCandidates { groups })
}

/// Les inscrites d'une poule, par ordre alphabétique — l'ordre où l'on cherche
/// une équipe pour la décocher.
fn group_candidates(
    team_ids: &[String],
    ctx: &DrawContext,
    played: &HashMap<TeamId, MatchesPlayed>,
) -> Vec<Candidate> {
    let (enrolled, _) = filter_enrolled_team_ids(team_ids, &ctx.teams);
    let mut teams: Vec<Candidate> = enrolled
        .iter()
        .filter_map(|id| ctx.teams.get(id))
        .map(|info| candidate(info, played))
        .collect();
    teams.sort_by_key(|team| team.team_name.to_lowercase());
    teams
}

/// Une inscrite, telle que l'écran la nomme, avec son nombre de matchs.
pub fn candidate(info: &TeamInfoDto, played: &HashMap<TeamId, MatchesPlayed>) -> Candidate {
    Candidate {
        team_id: info.team_id.clone(),
        team_name: info.team_name.clone(),
        coach_name: info.coach_name.clone(),
        roster_name: info.roster_name.clone(),
        matches_played: TeamId::try_new(&info.team_id)
            .ok()
            .and_then(|id| played.get(&id))
            .map_or(0, |count| count.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::group_repository_port::GroupWithTeams;
    use crate::app::competitions::domain::match_day::{
        MatchDay, MatchDayName, MatchDayPosition, MatchDayType, Pairing,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};

    fn day(day_type: MatchDayType, pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée".to_string()).unwrap(),
            day_type,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings,
        }
    }

    fn team(id: &TeamId, name: &str) -> TeamInfoDto {
        TeamInfoDto {
            team_id: id.to_string(),
            team_name: name.to_string(),
            coach_id: format!("coach-{name}"),
            coach_name: format!("Coach {name}"),
            roster_name: "Nains".to_string(),
            logo_url: None,
        }
    }

    fn group(name: &str, ids: &[&TeamId]) -> GroupWithTeams {
        GroupWithTeams {
            group_id: name.to_string(),
            group_name: name.to_string(),
            position: 0,
            team_ids: ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    fn context(
        match_day: MatchDay,
        teams: &[TeamInfoDto],
        groups: Vec<GroupWithTeams>,
    ) -> DrawContext {
        DrawContext {
            season_days: vec![match_day.clone()],
            match_day,
            teams: teams
                .iter()
                .map(|t| (t.team_id.clone(), t.clone()))
                .collect::<HashMap<_, _>>(),
            groups,
        }
    }

    fn pairing(home: &TeamId, away: &TeamId) -> Pairing {
        Pairing {
            id: PairingId::new(),
            home_team_id: *home,
            away_team_id: *away,
        }
    }

    #[test]
    fn a_rest_day_is_refused() {
        let ctx = context(day(MatchDayType::Rest, vec![]), &[], vec![]);

        assert!(matches!(
            candidates_of(&ctx),
            Err(DrawCandidatesError::Refused(
                DomainError::RestDayCannotBeDrawn
            ))
        ));
    }

    #[test]
    fn a_round_with_pairings_is_refused() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let ctx = context(
            day(MatchDayType::FixedDate, vec![pairing(&a, &b)]),
            &[],
            vec![],
        );

        assert!(matches!(
            candidates_of(&ctx),
            Err(DrawCandidatesError::Refused(
                DomainError::RoundAlreadyPaired
            ))
        ));
    }

    #[test]
    fn a_group_without_enrolled_team_does_not_appear() {
        let (a, b, gone) = (TeamId::new(), TeamId::new(), TeamId::new());
        let ctx = context(
            day(MatchDayType::FixedDate, vec![]),
            &[team(&a, "Alpha"), team(&b, "Beta")],
            vec![group("A", &[&a, &b]), group("B", &[&gone])],
        );

        let candidates = candidates_of(&ctx).unwrap();

        assert_eq!(candidates.groups.len(), 1);
        assert_eq!(candidates.groups[0].group_name, "A");
    }

    #[test]
    fn teams_are_sorted_by_name_and_unenrolled_ones_left_out() {
        let (a, b, gone) = (TeamId::new(), TeamId::new(), TeamId::new());
        let ctx = context(
            day(MatchDayType::FixedDate, vec![]),
            &[team(&a, "zèbres"), team(&b, "Aigles")],
            vec![group("A", &[&a, &gone, &b])],
        );

        let names: Vec<String> = candidates_of(&ctx).unwrap().groups[0]
            .teams
            .iter()
            .map(|t| t.team_name.clone())
            .collect();

        assert_eq!(names, vec!["Aigles", "zèbres"]);
    }

    /// R8 — le compte affiché est celui qui départagera l'exemption : les
    /// rencontres des autres journées, celle qu'on tire exclue.
    #[test]
    fn matches_played_count_the_other_rounds_only() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let drawn = day(MatchDayType::FixedDate, vec![]);
        let mut ctx = context(
            drawn,
            &[team(&a, "Alpha"), team(&b, "Beta")],
            vec![group("A", &[&a, &b])],
        );
        ctx.season_days
            .push(day(MatchDayType::FixedDate, vec![pairing(&a, &b)]));
        ctx.season_days
            .push(day(MatchDayType::FixedDate, vec![pairing(&a, &b)]));

        let teams = &candidates_of(&ctx).unwrap().groups[0].teams;

        assert!(teams.iter().all(|t| t.matches_played == 2));
    }
}
