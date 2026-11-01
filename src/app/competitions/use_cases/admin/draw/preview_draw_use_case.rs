//! L'aperçu du tirage : les rencontres tirées parmi les présentes, poule par
//! poule. **Rien n'est écrit** — l'écriture est `confirm_draw` (carte 618), qui
//! revérifie tout (R11).
//!
//! Le générateur est reçu : graine fixe en test, `StdRng::from_os_rng()` côté
//! handler.

use crate::app::competitions::domain::draw::{
    draw_pools, DrawInput, DrawProposal, History, MatchesPlayed,
};
use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::use_cases::admin::draw::draw_candidates_service::{
    candidate, Candidate,
};
use crate::app::competitions::use_cases::admin::draw::draw_input::{
    build_matches_played, load_draw_context, parse_team_ids, pool_input, DrawContext, DrawDeps,
    DrawLoadError,
};
use crate::app::competitions::use_cases::admin::team_enrollment::filter_enrolled_team_ids;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use rand::Rng;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct PreviewDrawCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
    pub present: Vec<TeamId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawPreview {
    pub groups: Vec<GroupPreview>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupPreview {
    pub group_name: String,
    pub pairings: Vec<PreviewPairing>,
    pub exempted: Option<Candidate>,
    pub absent: Vec<Candidate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewPairing {
    pub home: Candidate,
    pub away: Candidate,
    pub history: History,
}

#[derive(Debug)]
pub enum PreviewDrawError {
    RoundNotFound,
    IsRestDay,
    PairingsAlreadyExist,
    /// R10 — aucune poule n'a deux présentes.
    NotEnoughPresent,
    /// Un autre refus du domaine — le tirage ne devrait pas en produire.
    Refused(DomainError),
    Repository(String),
}

impl From<DrawLoadError> for PreviewDrawError {
    fn from(e: DrawLoadError) -> Self {
        match e {
            DrawLoadError::RoundNotFound => Self::RoundNotFound,
            DrawLoadError::Repository(msg) => Self::Repository(msg),
        }
    }
}

impl From<DomainError> for PreviewDrawError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::RestDayCannotBeDrawn => Self::IsRestDay,
            DomainError::RoundAlreadyPaired => Self::PairingsAlreadyExist,
            DomainError::NotEnoughPresent => Self::NotEnoughPresent,
            other => Self::Refused(other),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: PreviewDrawCommand,
    deps: &DrawDeps<'_>,
    rng: &mut impl Rng,
) -> Result<DrawPreview, PreviewDrawError> {
    let ctx =
        load_draw_context(&cmd.round_id.to_string(), &cmd.season_id.to_string(), deps).await?;
    preview_of(&ctx, &cmd.present, rng)
}

/// Le tirage d'une journée qui peut le recevoir, parmi les présentes inscrites.
/// Une fonction pure : l'aperçu ne tient aucun dépôt en écriture.
pub fn preview_of(
    ctx: &DrawContext,
    present: &[TeamId],
    rng: &mut impl Rng,
) -> Result<DrawPreview, PreviewDrawError> {
    ctx.match_day.can_receive_draw()?;
    let pools = pools_of(ctx, present);
    let inputs: Vec<DrawInput> = pools
        .iter()
        .map(|pool| pool_input(pool.present.clone(), ctx))
        .collect();
    let proposals = draw_pools(&inputs, rng)?;
    let played = build_matches_played(&ctx.season_days, &ctx.match_day.id.to_string());
    let groups = pools
        .into_iter()
        .zip(proposals)
        .map(|(pool, proposal)| group_preview(pool, proposal, ctx, &played))
        .collect();
    Ok(DrawPreview { groups })
}

/// Une poule à tirer : ses présentes et ses absentes, parmi ses inscrites.
struct Pool {
    name: String,
    present: Vec<TeamId>,
    absent: Vec<TeamId>,
}

/// Les poules qui ont au moins une inscrite — les autres n'ont rien à montrer.
fn pools_of(ctx: &DrawContext, present: &[TeamId]) -> Vec<Pool> {
    let present: HashSet<&TeamId> = present.iter().collect();
    ctx.groups
        .iter()
        .map(|group| {
            let (enrolled, _) = filter_enrolled_team_ids(&group.team_ids, &ctx.teams);
            let (here, away): (Vec<TeamId>, Vec<TeamId>) = parse_team_ids(enrolled.iter())
                .into_iter()
                .partition(|team| present.contains(team));
            Pool {
                name: group.group_name.clone(),
                present: here,
                absent: away,
            }
        })
        .filter(|pool| !pool.present.is_empty() || !pool.absent.is_empty())
        .collect()
}

fn group_preview(
    pool: Pool,
    proposal: DrawProposal,
    ctx: &DrawContext,
    played: &HashMap<TeamId, MatchesPlayed>,
) -> GroupPreview {
    let named = |team: &TeamId| named_candidate(team, ctx, played);
    GroupPreview {
        group_name: pool.name,
        pairings: proposal
            .pairings
            .into_iter()
            .map(|p| PreviewPairing {
                home: named(&p.home),
                away: named(&p.away),
                history: p.history,
            })
            .collect(),
        exempted: proposal.exempted.as_ref().map(named),
        absent: pool.absent.iter().map(named).collect(),
    }
}

/// Toute équipe tirée est une inscrite, donc connue de `ctx.teams`.
fn named_candidate(
    team: &TeamId,
    ctx: &DrawContext,
    played: &HashMap<TeamId, MatchesPlayed>,
) -> Candidate {
    ctx.teams
        .get(&team.to_string())
        .map(|info| candidate(info, played))
        .unwrap_or_else(|| Candidate {
            team_id: team.to_string(),
            team_name: team.to_string(),
            coach_name: String::new(),
            roster_name: String::new(),
            matches_played: played.get(team).map_or(0, |count| count.0),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::group_repository_port::GroupWithTeams;
    use crate::app::competitions::domain::match_day::{
        MatchDay, MatchDayName, MatchDayPosition, MatchDayType, Pairing,
    };
    use crate::app::competitions::ports::TeamInfoDto;
    use crate::app::shared_kernel::bloodbowl::ids::PairingId;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

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

    fn group(name: &str, ids: &[TeamId]) -> GroupWithTeams {
        GroupWithTeams {
            group_id: name.to_string(),
            group_name: name.to_string(),
            position: 0,
            team_ids: ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    fn teams(n: usize) -> Vec<TeamId> {
        (0..n).map(|_| TeamId::new()).collect()
    }

    /// Une journée vide, deux poules de quatre inscrites.
    fn two_pools() -> (DrawContext, Vec<TeamId>, Vec<TeamId>) {
        let (a, b) = (teams(4), teams(4));
        let infos: Vec<TeamInfoDto> = a
            .iter()
            .chain(&b)
            .enumerate()
            .map(|(i, id)| team(id, &format!("Équipe {i}")))
            .collect();
        let match_day = day(MatchDayType::FixedDate, vec![]);
        let ctx = DrawContext {
            season_days: vec![match_day.clone()],
            match_day,
            teams: infos.into_iter().map(|t| (t.team_id.clone(), t)).collect(),
            groups: vec![group("A", &a), group("B", &b)],
        };
        (ctx, a, b)
    }

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn drawn_ids(group: &GroupPreview) -> HashSet<String> {
        group
            .pairings
            .iter()
            .flat_map(|p| [p.home.team_id.clone(), p.away.team_id.clone()])
            .collect()
    }

    /// R3 — chaque poule est tirée pour elle-même : aucune rencontre ne mêle
    /// deux poules.
    #[test]
    fn each_pool_is_drawn_on_its_own() {
        let (ctx, a, b) = two_pools();
        let present: Vec<TeamId> = a.iter().chain(&b).copied().collect();

        let preview = preview_of(&ctx, &present, &mut rng()).unwrap();

        let ids = |teams: &[TeamId]| teams.iter().map(|t| t.to_string()).collect::<HashSet<_>>();
        assert_eq!(drawn_ids(&preview.groups[0]), ids(&a));
        assert_eq!(drawn_ids(&preview.groups[1]), ids(&b));
    }

    #[test]
    fn absent_teams_are_left_out_and_listed() {
        let (ctx, a, b) = two_pools();
        let present: Vec<TeamId> = a[..3].iter().chain(&b).copied().collect();

        let preview = preview_of(&ctx, &present, &mut rng()).unwrap();

        let pool_a = &preview.groups[0];
        assert!(!drawn_ids(pool_a).contains(&a[3].to_string()));
        assert_eq!(pool_a.absent.len(), 1);
        assert_eq!(pool_a.absent[0].team_id, a[3].to_string());
        assert_eq!(pool_a.pairings.len(), 1, "trois présentes : une rencontre");
        assert!(pool_a.exempted.is_some(), "et une exemptée");
    }

    /// R10 — aucune poule à deux présentes : rien à tirer.
    #[test]
    fn no_drawable_pool_is_refused() {
        let (ctx, a, b) = two_pools();

        let refus = preview_of(&ctx, &[a[0], b[0]], &mut rng());

        assert!(matches!(refus, Err(PreviewDrawError::NotEnoughPresent)));
    }

    #[test]
    fn a_round_with_pairings_is_refused() {
        let (mut ctx, a, b) = two_pools();
        ctx.match_day.pairings.push(Pairing {
            id: PairingId::new(),
            home_team_id: a[0],
            away_team_id: a[1],
        });
        let present: Vec<TeamId> = a.iter().chain(&b).copied().collect();

        let refus = preview_of(&ctx, &present, &mut rng());

        assert!(matches!(refus, Err(PreviewDrawError::PairingsAlreadyExist)));
    }

    #[test]
    fn a_fixed_seed_gives_the_same_preview() {
        let (ctx, a, b) = two_pools();
        let present: Vec<TeamId> = a.iter().chain(&b).copied().collect();

        let first = preview_of(&ctx, &present, &mut rng()).unwrap();
        let second = preview_of(&ctx, &present, &mut rng()).unwrap();

        assert_eq!(first, second);
    }

    /// L'aperçu ne tient aucun dépôt : le contexte qu'il lit sort inchangé —
    /// la journée tirée reste vide.
    #[test]
    fn nothing_is_written_to_the_round() {
        let (ctx, a, b) = two_pools();
        let present: Vec<TeamId> = a.iter().chain(&b).copied().collect();

        preview_of(&ctx, &present, &mut rng()).unwrap();

        assert!(ctx.match_day.pairings.is_empty());
    }
}
