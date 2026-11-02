//! Le tirage au sort du MVP (carte 638, R8).
//!
//! Le coach désigne **six** joueurs qui ont joué, distincts ; le serveur en
//! tire un. Le générateur est **reçu** (`&mut impl Rng`), comme pour le tirage
//! du calendrier (`competitions/domain/draw.rs`) : `StdRng::from_os_rng()` dans
//! le contrôleur, une graine fixe en test.

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::value_objects::{ActionId, ActionPlayer, TeamSide};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Le nombre de joueurs parmi lesquels le MVP est tiré.
pub const DRAW_SIZE: usize = 6;

/// Un joueur désigné pour le tirage, avec son nom et son poste **au moment du
/// tirage** : l'écran les relit tels quels, même si le joueur est renommé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignatedPlayer {
    pub player: ActionPlayer,
    pub display_name: String, // arch:ok instantané
    /// `serde(default)` par prudence : le poste n'est qu'un libellé, et
    /// l'action tirée le porte déjà.
    #[serde(default)]
    pub position: String, // arch:ok instantané
}

/// Un MVP tiré au sort : l'action qui l'inscrit, et les six désignés. Le
/// tirage est définitif — `delete_action` refuse cette action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawnMvp {
    pub action_id: ActionId,
    pub team_side: TeamSide,
    pub designated: Vec<DesignatedPlayer>,
}

impl DrawnMvp {
    /// L'événement qui le retient : le même, à la saisie et au rejeu.
    pub fn to_event(&self) -> MatchReportDomainEvent {
        MatchReportDomainEvent::MvpDrawn {
            action_id: self.action_id.clone(),
            team_side: self.team_side,
            designated: self.designated.clone(),
        }
    }

    /// Les noms des désignés, dans l'ordre de leur désignation.
    pub fn designated_names(&self) -> Vec<&str> {
        self.designated
            .iter()
            .map(|d| d.display_name.as_str())
            .collect()
    }
}

/// Exactement six désignés, tous distincts.
pub fn check_designated(designated: &[DesignatedPlayer]) -> Result<(), DomainError> {
    if designated.len() != DRAW_SIZE {
        return Err(DomainError::DrawNeedsSixPlayers(designated.len()));
    }
    let duplicated = designated
        .iter()
        .enumerate()
        .any(|(i, d)| designated[..i].iter().any(|p| p.player == d.player));
    if duplicated {
        return Err(DomainError::DuplicateDesignatedPlayer);
    }
    Ok(())
}

/// Le tiré, un des désignés. Appelée sur une liste vérifiée : jamais vide.
pub fn pick<'a>(designated: &'a [DesignatedPlayer], rng: &mut impl Rng) -> &'a DesignatedPlayer {
    &designated[rng.random_range(0..designated.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn designated(n: usize) -> Vec<DesignatedPlayer> {
        (0..n)
            .map(|i| DesignatedPlayer {
                player: ActionPlayer::Regular(PlayerId::new()),
                display_name: format!("Nain {i}"),
                position: "Bloqueur".into(),
            })
            .collect()
    }

    #[test]
    fn six_designes_distincts_sont_acceptes() {
        assert_eq!(check_designated(&designated(6)), Ok(()));
    }

    #[test]
    fn cinq_ou_sept_designes_sont_refuses() {
        assert_eq!(
            check_designated(&designated(5)),
            Err(DomainError::DrawNeedsSixPlayers(5))
        );
        assert_eq!(
            check_designated(&designated(7)),
            Err(DomainError::DrawNeedsSixPlayers(7))
        );
    }

    #[test]
    fn un_joueur_designe_deux_fois_est_refuse() {
        let mut list = designated(6);
        list[5] = list[2].clone();
        assert_eq!(
            check_designated(&list),
            Err(DomainError::DuplicateDesignatedPlayer)
        );
    }

    /// Sur une graine fixe, le tiré est toujours l'un des six — et chacun peut
    /// sortir.
    #[test]
    fn le_tire_est_toujours_l_un_des_six() {
        let list = designated(6);
        let mut rng = StdRng::seed_from_u64(638);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            let winner = pick(&list, &mut rng);
            assert!(list.contains(winner));
            seen.insert(winner.display_name.clone());
        }
        assert_eq!(seen.len(), DRAW_SIZE);
    }

    #[test]
    fn une_meme_graine_tire_le_meme_joueur() {
        let list = designated(6);
        let a = pick(&list, &mut StdRng::seed_from_u64(7)).clone();
        let b = pick(&list, &mut StdRng::seed_from_u64(7)).clone();
        assert_eq!(a, b);
    }
}
