//! Le prochain adversaire d'une équipe (R2, R12 — carte 599).
//!
//! Une équipe n'est inscrite qu'à une saison : son prochain adversaire est
//! celui du match **en cours de saisie** s'il y en a un, sinon du premier match
//! **à venir** dans l'ordre des journées. Les matchs joués ne comptent pas.
//!
//! La requête SQL trie déjà dans cet ordre ; la règle n'en dépend pas pour
//! autant — un tri changé pour un autre écran ne doit pas changer l'adversaire
//! imprimé.

use crate::app::competitions::domain::match_day::{MatchDayPosition, Pairing};
use crate::app::shared_kernel::bloodbowl::ids::MatchId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingStatus {
    InProgress,
    Upcoming,
    Played,
}

impl PairingStatus {
    /// Depuis le statut porté par `competition_match_display_proj` — les trois
    /// valeurs qu'y écrivent les listeners, et elles seules.
    ///
    /// **Pas de joker** : un statut ajouté demain (« reporté »…) rangé d'office
    /// parmi les matchs joués ferait imprimer l'adversaire du match suivant,
    /// sans un mot. Inconnu, il rend `None`, et l'appelant le signale.
    pub fn from_projection(status: &str) -> Option<Self> {
        match status {
            "in_progress" => Some(Self::InProgress),
            "upcoming" => Some(Self::Upcoming),
            "completed" => Some(Self::Played),
            _ => None,
        }
    }
}

pub struct ScheduledPairing {
    pub round_id: MatchId,
    pub round_position: MatchDayPosition,
    pub pairing: Pairing,
    pub status: PairingStatus,
}

/// Où joue l'adversaire. Sert au favori à égalité de VE (R10) : l'équipe à
/// domicile l'emporte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Venue {
    Home,
    Away,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextOpponent {
    pub team_id: TeamId,
    pub round_id: MatchId,
    pub opponent_venue: Venue,
}

/// Le prochain adversaire : le premier des matchs à venir (R12).
pub fn next_opponent(team: &TeamId, pairings: &[ScheduledPairing]) -> Option<NextOpponent> {
    upcoming_opponents(team, pairings).into_iter().next()
}

/// Tous les matchs que l'équipe n'a pas encore joués, dans l'ordre de R12 : le
/// match en cours de saisie d'abord, puis les journées à venir dans leur
/// ordre. Le coach peut en choisir un autre que le premier — un match oublié
/// d'une journée passée reste le prochain à jouer, mais pas forcément celui
/// qu'il veut imprimer.
pub fn upcoming_opponents(team: &TeamId, pairings: &[ScheduledPairing]) -> Vec<NextOpponent> {
    let mut upcoming: Vec<&ScheduledPairing> = pairings
        .iter()
        .filter(|p| p.status != PairingStatus::Played && p.pairing.engage(team))
        .collect();
    upcoming.sort_by_key(|p| {
        (
            p.status != PairingStatus::InProgress,
            p.round_position.into_inner(),
        )
    });
    upcoming
        .into_iter()
        .filter_map(|p| opponent_in(team, p))
        .collect()
}

fn opponent_in(team: &TeamId, scheduled: &ScheduledPairing) -> Option<NextOpponent> {
    let opponent = scheduled.pairing.adversaire_de(team)?;
    Some(NextOpponent {
        team_id: opponent.clone(),
        round_id: scheduled.round_id.clone(),
        opponent_venue: if scheduled.pairing.home_team_id == *opponent {
            Venue::Home
        } else {
            Venue::Away
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::PairingId;

    fn rencontre(
        domicile: &TeamId,
        exterieur: &TeamId,
        position: i32,
        status: PairingStatus,
    ) -> ScheduledPairing {
        ScheduledPairing {
            round_id: MatchId::new(),
            round_position: MatchDayPosition::try_new(position).unwrap(),
            pairing: Pairing {
                id: PairingId::new(),
                home_team_id: domicile.clone(),
                away_team_id: exterieur.clone(),
            },
            status,
        }
    }

    #[test]
    fn le_match_en_cours_passe_avant_le_match_a_venir() {
        let (moi, a, b) = (TeamId::new(), TeamId::new(), TeamId::new());
        let matchs = [
            rencontre(&moi, &a, 2, PairingStatus::Upcoming),
            rencontre(&b, &moi, 5, PairingStatus::InProgress),
        ];
        let prochain = next_opponent(&moi, &matchs).unwrap();
        assert_eq!(prochain.team_id, b);
        assert_eq!(prochain.opponent_venue, Venue::Home);
    }

    #[test]
    fn la_journee_la_plus_proche_d_abord_et_les_matchs_joues_ignores() {
        let (moi, a, b, c) = (TeamId::new(), TeamId::new(), TeamId::new(), TeamId::new());
        let matchs = [
            rencontre(&moi, &a, 1, PairingStatus::Played),
            rencontre(&moi, &b, 7, PairingStatus::Upcoming),
            rencontre(&moi, &c, 3, PairingStatus::Upcoming),
        ];
        let prochain = next_opponent(&moi, &matchs).unwrap();
        assert_eq!(prochain.team_id, c);
        assert_eq!(prochain.round_id, matchs[2].round_id);
        assert_eq!(prochain.opponent_venue, Venue::Away);
    }

    /// La liste suit l'ordre de R12, et son premier élément est le prochain
    /// adversaire : une seule règle, deux usages.
    #[test]
    fn les_matchs_a_venir_dans_l_ordre_de_r12() {
        let (moi, a, b, c, d) = (
            TeamId::new(),
            TeamId::new(),
            TeamId::new(),
            TeamId::new(),
            TeamId::new(),
        );
        let matchs = [
            rencontre(&moi, &a, 1, PairingStatus::Played),
            rencontre(&moi, &b, 6, PairingStatus::Upcoming),
            rencontre(&c, &moi, 3, PairingStatus::Upcoming),
            rencontre(&moi, &d, 5, PairingStatus::InProgress),
        ];
        let equipes: Vec<TeamId> = upcoming_opponents(&moi, &matchs)
            .into_iter()
            .map(|o| o.team_id)
            .collect();
        assert_eq!(equipes, vec![d, c, b]);
        assert_eq!(
            next_opponent(&moi, &matchs).map(|o| o.team_id),
            Some(equipes[0].clone())
        );
    }

    #[test]
    fn sans_match_a_venir_aucun_adversaire() {
        let (moi, a) = (TeamId::new(), TeamId::new());
        let matchs = [rencontre(&moi, &a, 1, PairingStatus::Played)];
        assert_eq!(next_opponent(&moi, &matchs), None);
        assert_eq!(next_opponent(&moi, &[]), None);
    }

    #[test]
    fn une_rencontre_ou_l_equipe_ne_joue_pas_est_ignoree() {
        let (moi, a, b) = (TeamId::new(), TeamId::new(), TeamId::new());
        let matchs = [rencontre(&a, &b, 1, PairingStatus::Upcoming)];
        assert_eq!(next_opponent(&moi, &matchs), None);
    }

    /// Les valeurs que la projection contient réellement : `'completed'` est
    /// celle de `match_report_published_listener`.
    #[test]
    fn le_statut_de_la_projection_est_lu() {
        assert_eq!(
            PairingStatus::from_projection("in_progress"),
            Some(PairingStatus::InProgress)
        );
        assert_eq!(
            PairingStatus::from_projection("upcoming"),
            Some(PairingStatus::Upcoming)
        );
        assert_eq!(
            PairingStatus::from_projection("completed"),
            Some(PairingStatus::Played)
        );
    }

    #[test]
    fn un_statut_inconnu_n_est_pas_range_parmi_les_matchs_joues() {
        assert_eq!(PairingStatus::from_projection("postponed"), None);
        assert_eq!(PairingStatus::from_projection("played"), None);
    }
}
