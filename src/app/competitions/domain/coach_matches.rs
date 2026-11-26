//! Les matchs à jouer d'un coach (carte 675, besoins I7-B1 et I7-B2).
//!
//! Toutes les rencontres non jouées de ses équipes inscrites, **une fois
//! chacune** — deux de ses équipes peuvent se rencontrer —, avec le prochain
//! match de chaque équipe marqué.
//!
//! Le prochain match d'une équipe est celui de `next_opponent` : la même
//! règle (R12), pas une seconde. Un match d'une journée passée qui n'a pas été
//! joué reste donc le prochain, quelle que soit sa date.
//!
//! L'ordre d'affichage, lui, est propre à cette liste : le match en cours de
//! saisie d'abord, puis par date limite de journée croissante, les journées
//! sans date en dernier, et à égalité par position de journée.

use crate::app::competitions::domain::next_opponent::{
    next_opponent, PairingStatus, ScheduledPairing,
};
use crate::app::shared_kernel::bloodbowl::ids::PairingId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;

/// Les rencontres d'une équipe du coach, sur sa saison, tous statuts confondus.
pub struct TeamSchedule {
    pub team: TeamId,
    pub pairings: Vec<ScheduledPairing>,
}

/// Une rencontre à jouer, et si elle est le prochain match d'au moins une des
/// équipes du coach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchToPlay {
    pub pairing_id: PairingId,
    pub is_next: bool,
}

/// Les matchs à jouer, sans doublon, triés, le prochain de chaque équipe marqué.
pub fn matches_to_play(schedules: &[TeamSchedule]) -> Vec<MatchToPlay> {
    let next: Vec<PairingId> = schedules.iter().filter_map(next_pairing_of).collect();
    let mut unplayed = unplayed_once(schedules);
    unplayed.sort_by_key(|p| display_key(p));
    unplayed
        .into_iter()
        .map(|p| MatchToPlay {
            pairing_id: p.pairing.id.clone(),
            is_next: next.contains(&p.pairing.id),
        })
        .collect()
}

/// La rencontre du prochain match de l'équipe, telle que `next_opponent` la
/// désigne : sa journée et son adversaire.
fn next_pairing_of(schedule: &TeamSchedule) -> Option<PairingId> {
    let next = next_opponent(&schedule.team, &schedule.pairings)?;
    schedule
        .pairings
        .iter()
        .find(|p| {
            p.round_id == next.round_id
                && p.pairing.adversaire_de(&schedule.team) == Some(&next.team_id)
        })
        .map(|p| p.pairing.id.clone())
}

/// Les rencontres non jouées de toutes les équipes, chacune une seule fois :
/// celle qui oppose deux équipes du coach figure dans les deux calendriers.
fn unplayed_once(schedules: &[TeamSchedule]) -> Vec<&ScheduledPairing> {
    let mut seen: Vec<&ScheduledPairing> = Vec::new();
    for pairing in schedules.iter().flat_map(|s| s.pairings.iter()) {
        let already = seen.iter().any(|p| p.pairing.id == pairing.pairing.id);
        if pairing.status != PairingStatus::Played && !already {
            seen.push(pairing);
        }
    }
    seen
}

/// En cours d'abord ; puis datées avant non datées ; puis par date limite —
/// `AAAA-MM-JJ`, que l'ordre des chaînes suffit à trier — ; puis par position.
fn display_key(pairing: &ScheduledPairing) -> (bool, bool, String, i32) {
    let deadline = pairing.deadline.as_ref().map(|d| d.to_string());
    (
        pairing.status != PairingStatus::InProgress,
        deadline.is_none(),
        deadline.unwrap_or_default(),
        pairing.round_position.into_inner(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{MatchDayPosition, Pairing};
    use crate::app::shared_kernel::bloodbowl::date_string::DateString;
    use crate::app::shared_kernel::bloodbowl::ids::MatchId;

    fn pairing(
        home: &TeamId,
        away: &TeamId,
        position: i32,
        status: PairingStatus,
        deadline: Option<&str>,
    ) -> ScheduledPairing {
        ScheduledPairing {
            round_id: MatchId::new(),
            round_position: MatchDayPosition::try_new(position).unwrap(),
            pairing: Pairing {
                id: PairingId::new(),
                home_team_id: home.clone(),
                away_team_id: away.clone(),
            },
            status,
            deadline: deadline.map(|d| DateString::try_new(d).unwrap()),
        }
    }

    fn schedule(team: &TeamId, pairings: &[&ScheduledPairing]) -> TeamSchedule {
        TeamSchedule {
            team: team.clone(),
            pairings: pairings.iter().map(|p| (*p).clone()).collect(),
        }
    }

    fn ids(matches: &[MatchToPlay]) -> Vec<PairingId> {
        matches.iter().map(|m| m.pairing_id.clone()).collect()
    }

    fn next_ids(matches: &[MatchToPlay]) -> Vec<PairingId> {
        matches
            .iter()
            .filter(|m| m.is_next)
            .map(|m| m.pairing_id.clone())
            .collect()
    }

    /// Deux équipes du coach qui se rencontrent : la rencontre n'apparaît
    /// qu'une fois, et elle est le prochain match des deux.
    #[test]
    fn a_pairing_between_two_coach_teams_appears_once() {
        let (mine_a, mine_b) = (TeamId::new(), TeamId::new());
        let derby = pairing(&mine_a, &mine_b, 1, PairingStatus::Upcoming, None);
        let schedules = [schedule(&mine_a, &[&derby]), schedule(&mine_b, &[&derby])];
        let matches = matches_to_play(&schedules);
        assert_eq!(ids(&matches), vec![derby.pairing.id.clone()]);
        assert!(matches[0].is_next);
    }

    /// Chaque équipe a son prochain match marqué, et lui seul.
    #[test]
    fn the_next_match_of_each_team_is_marked() {
        let (mine_a, mine_b, x, y) = (TeamId::new(), TeamId::new(), TeamId::new(), TeamId::new());
        let a1 = pairing(&mine_a, &x, 1, PairingStatus::Upcoming, None);
        let a2 = pairing(&mine_a, &y, 2, PairingStatus::Upcoming, None);
        let b1 = pairing(&y, &mine_b, 3, PairingStatus::Upcoming, None);
        let b2 = pairing(&x, &mine_b, 4, PairingStatus::Upcoming, None);
        let schedules = [
            schedule(&mine_a, &[&a2, &a1]),
            schedule(&mine_b, &[&b2, &b1]),
        ];
        let matches = matches_to_play(&schedules);
        assert_eq!(matches.len(), 4);
        assert_eq!(
            next_ids(&matches),
            vec![a1.pairing.id.clone(), b1.pairing.id.clone()]
        );
    }

    /// Un match en retard, daté loin dans le passé ou sans date, reste le
    /// prochain : c'est la règle de `next_opponent`, pas celle de la date.
    #[test]
    fn a_distant_match_stays_the_next_one() {
        let (mine, x, y) = (TeamId::new(), TeamId::new(), TeamId::new());
        let late = pairing(&mine, &x, 1, PairingStatus::Upcoming, None);
        let soon = pairing(&mine, &y, 2, PairingStatus::Upcoming, Some("2026-01-10"));
        let matches = matches_to_play(&[schedule(&mine, &[&soon, &late])]);
        assert_eq!(next_ids(&matches), vec![late.pairing.id.clone()]);
        assert_eq!(
            ids(&matches),
            vec![soon.pairing.id.clone(), late.pairing.id.clone()]
        );
    }

    /// Le prochain match d'une équipe est le même que `next_opponent` : le
    /// match en cours passe avant une journée plus proche.
    #[test]
    fn the_match_in_progress_is_the_next_one() {
        let (mine, x, y) = (TeamId::new(), TeamId::new(), TeamId::new());
        let upcoming = pairing(&mine, &x, 1, PairingStatus::Upcoming, None);
        let in_progress = pairing(&y, &mine, 5, PairingStatus::InProgress, None);
        let matches = matches_to_play(&[schedule(&mine, &[&upcoming, &in_progress])]);
        assert_eq!(next_ids(&matches), vec![in_progress.pairing.id.clone()]);
    }

    /// En cours, puis datés par date limite croissante, puis sans date ; à
    /// égalité de date, par position de journée.
    #[test]
    fn matches_are_sorted_in_progress_then_dated_then_undated() {
        let (mine, o) = (TeamId::new(), TeamId::new());
        let undated = pairing(&mine, &o, 1, PairingStatus::Upcoming, None);
        let late = pairing(&mine, &o, 2, PairingStatus::Upcoming, Some("2026-12-20"));
        let early_6 = pairing(&mine, &o, 6, PairingStatus::Upcoming, Some("2026-12-01"));
        let early_3 = pairing(&mine, &o, 3, PairingStatus::Upcoming, Some("2026-12-01"));
        let live = pairing(&mine, &o, 9, PairingStatus::InProgress, Some("2027-01-01"));
        let all = [&undated, &late, &early_6, &early_3, &live];
        let matches = matches_to_play(&[schedule(&mine, &all)]);
        let expected = [&live, &early_3, &early_6, &late, &undated];
        let expected: Vec<PairingId> = expected.iter().map(|p| p.pairing.id.clone()).collect();
        assert_eq!(ids(&matches), expected);
    }

    #[test]
    fn a_played_match_is_excluded() {
        let (mine, x, y) = (TeamId::new(), TeamId::new(), TeamId::new());
        let played = pairing(&mine, &x, 1, PairingStatus::Played, Some("2026-01-01"));
        let upcoming = pairing(&mine, &y, 2, PairingStatus::Upcoming, None);
        let matches = matches_to_play(&[schedule(&mine, &[&played, &upcoming])]);
        assert_eq!(ids(&matches), vec![upcoming.pairing.id.clone()]);
        assert!(matches[0].is_next);
    }

    #[test]
    fn a_team_without_match_to_play_gives_nothing() {
        let (mine, x) = (TeamId::new(), TeamId::new());
        let played = pairing(&mine, &x, 1, PairingStatus::Played, None);
        assert!(matches_to_play(&[schedule(&mine, &[&played])]).is_empty());
        assert!(matches_to_play(&[]).is_empty());
    }

    /// Le prochain d'une équipe désigne sa rencontre, pas une autre de la même
    /// journée ; sans match à jouer, il n'y en a pas.
    #[test]
    fn next_pairing_of_designates_the_pairing_of_next_opponent() {
        let (mine, x, y, z) = (TeamId::new(), TeamId::new(), TeamId::new(), TeamId::new());
        let mine_vs_x = pairing(&mine, &x, 1, PairingStatus::Upcoming, None);
        let mut other = pairing(&y, &z, 1, PairingStatus::Upcoming, None);
        other.round_id = mine_vs_x.round_id.clone();
        let s = schedule(&mine, &[&other, &mine_vs_x]);
        assert_eq!(next_pairing_of(&s), Some(mine_vs_x.pairing.id.clone()));
        let played = pairing(&mine, &x, 2, PairingStatus::Played, None);
        assert_eq!(next_pairing_of(&schedule(&mine, &[&played])), None);
    }

    #[test]
    fn unplayed_once_keeps_the_first_occurrence_and_drops_played() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let shared = pairing(&a, &b, 1, PairingStatus::Upcoming, None);
        let played = pairing(&a, &b, 2, PairingStatus::Played, None);
        let schedules = [schedule(&a, &[&shared, &played]), schedule(&b, &[&shared])];
        let kept: Vec<&PairingId> = unplayed_once(&schedules)
            .iter()
            .map(|p| &p.pairing.id)
            .collect();
        assert_eq!(kept, vec![&shared.pairing.id]);
    }

    #[test]
    fn display_key_puts_undated_after_dated() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let dated = pairing(&a, &b, 7, PairingStatus::Upcoming, Some("2026-12-01"));
        let undated = pairing(&a, &b, 1, PairingStatus::Upcoming, None);
        assert!(display_key(&dated) < display_key(&undated));
        let live = pairing(&a, &b, 9, PairingStatus::InProgress, None);
        assert!(display_key(&live) < display_key(&dated));
    }
}
