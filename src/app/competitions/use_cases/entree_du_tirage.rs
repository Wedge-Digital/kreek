//! Ce qui compose l'entrée du tirage, pour les **deux** chemins qui l'appellent.
//!
//! Le Calendrier (`admin::generate_pairings`) et le sondage de présence
//! (`presences::draw_pairings_use_case`) construisent le même `DrawInput` à
//! partir des mêmes données. Les recopier ferait deux tirages qui divergeraient —
//! ce que la carte 541 vient de réparer d'un seul côté, en découvrant que R9
//! n'était alimentée nulle part.
//!
//! Un module à part et non des fonctions publiques dans l'un des deux use cases :
//! aucun n'a à importer l'autre.
//!
//! **La matière de R10 est inter-BC, la règle est métier.** La relation équipe →
//! coach arrive par `ITeamInfoPort` ; le domaine reçoit des paires interdites, il
//! n'a pas à savoir qu'un coach existe.

use crate::app::competitions::domain::match_day::MatchDay;
use crate::app::competitions::domain::tirage::{paire, NombreDeMatchs, RencontresJouees};
use crate::app::competitions::ports::TeamInfoDto;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use std::collections::{HashMap, HashSet};

pub fn build_interdites(
    equipes: &[TeamId],
    team_display: &HashMap<String, TeamInfoDto>,
) -> HashSet<(TeamId, TeamId)> {
    let mut interdites = HashSet::new();
    for (rang, a) in equipes.iter().enumerate() {
        for b in equipes.iter().skip(rang + 1) {
            if meme_coach(a, b, team_display) {
                interdites.insert(paire(a, b));
            }
        }
    }
    interdites
}

pub fn meme_coach(a: &TeamId, b: &TeamId, team_display: &HashMap<String, TeamInfoDto>) -> bool {
    match (
        team_display.get(&a.to_string()),
        team_display.get(&b.to_string()),
    ) {
        (Some(x), Some(y)) => x.coach_id == y.coach_id,
        _ => false,
    }
}

/// L'historique de la saison, **compté** par paire, la journée en cours exclue.
///
/// Un `HashSet` ne disait que « déjà jouée » : c'est ce choix de type qui
/// rendait la minimisation de R8.2 impossible.
pub fn build_historique(days: &[MatchDay], exclude_id: &str) -> RencontresJouees {
    let mut historique = RencontresJouees::new();
    for day in days.iter().filter(|d| d.id.to_string() != exclude_id) {
        for p in &day.pairings {
            historique.enregistrer(
                &p.home_team_id,
                &p.away_team_id,
                day.position,
                day.name.clone(),
            );
        }
    }
    historique
}

/// R9 — le nombre d'appariements **programmés** de chaque équipe sur la saison.
///
/// Même boucle que `build_historique`, même exclusion de la journée tirée : les
/// appariements qu'on s'apprête à écrire ne comptent pas contre les équipes
/// qu'ils concernent. Une fonction séparée plutôt qu'un tuple — chacune reste
/// courte et porte son nom.
///
/// Les appariements et non les rapports de match : ils vivent dans les tables du
/// BC, donc le compte se lit sans port.
pub fn build_matchs_joues(days: &[MatchDay], exclude_id: &str) -> HashMap<TeamId, NombreDeMatchs> {
    let mut comptes: HashMap<TeamId, NombreDeMatchs> = HashMap::new();
    for day in days.iter().filter(|d| d.id.to_string() != exclude_id) {
        for p in &day.pairings {
            for camp in [p.home_team_id, p.away_team_id] {
                comptes.entry(camp).or_default().0 += 1;
            }
        }
    }
    comptes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::competitions::domain::match_day::{
        MatchDayName, MatchDayPosition, MatchDayType, Pairing,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};

    fn match_day_with_pairings(pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 1".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(0).unwrap(),
            pairings,
        }
    }

    // ── R9 — le compte qui alimente le critère d'exemption (carte 541) ───────

    fn appariement(home: &TeamId, away: &TeamId) -> Pairing {
        Pairing {
            id: PairingId::new(),
            home_team_id: *home,
            away_team_id: *away,
        }
    }

    #[test]
    fn build_matchs_joues_compte_les_deux_camps() {
        let (a, b, c) = (TeamId::new(), TeamId::new(), TeamId::new());
        let jours = vec![
            match_day_with_pairings(vec![appariement(&a, &b)]),
            match_day_with_pairings(vec![appariement(&a, &c)]),
        ];

        let comptes = build_matchs_joues(&jours, "aucune");

        assert_eq!(comptes.get(&a), Some(&NombreDeMatchs(2)));
        assert_eq!(comptes.get(&b), Some(&NombreDeMatchs(1)));
        assert_eq!(comptes.get(&c), Some(&NombreDeMatchs(1)));
    }

    /// Les appariements qu'on s'apprête à écrire ne comptent pas contre les
    /// équipes qu'ils concernent — même exclusion que `build_historique`.
    #[test]
    fn build_matchs_joues_ignore_la_journee_tiree() {
        let (a, b) = (TeamId::new(), TeamId::new());
        let jours = vec![
            match_day_with_pairings(vec![appariement(&a, &b)]),
            match_day_with_pairings(vec![appariement(&a, &b)]),
        ];
        let tiree = jours[1].id.to_string();

        let comptes = build_matchs_joues(&jours, &tiree);

        assert_eq!(comptes.get(&a), Some(&NombreDeMatchs(1)));
    }

    /// Le défaut que la 541 corrige : le champ arrivait vide au tirage, donc le
    /// critère ne départageait rien. Ce test échouerait sur le code d'avant.
    #[test]
    fn une_saison_vierge_ne_donne_aucun_compte() {
        let comptes = build_matchs_joues(&[], "aucune");

        assert!(
            comptes.is_empty(),
            "aucune journée, aucun match — et toutes les équipes à égalité"
        );
    }
}
