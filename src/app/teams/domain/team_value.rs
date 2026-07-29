use crate::app::shared_kernel::bloodbowl::staff_counts::{
    ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
};
use crate::app::teams::domain::value_objects::Kpo;

/// Effectif complet aligné au coup d'envoi. En deçà, l'équipe est complétée par
/// des journaliers — c'est la même règle que
/// `match_report::init_temp_players_use_case::collect_journeymen`.
const MATCH_SQUAD_SIZE: u32 = 11;

pub struct ValuedPlayer {
    pub value_kpo: Kpo,
    pub available_for_next_match: bool,
}

pub struct TeamValueInputs {
    pub players: Vec<ValuedPlayer>,
    pub rerolls: RerollCount,
    pub reroll_price: Kpo,
    pub apothecaries: ApothecaryCount,
    pub apothecary_price: Kpo,
    pub assistants: AssistantCount,
    pub assistant_price: Kpo,
    pub cheerleaders: CheerleaderCount,
    pub cheerleader_price: Kpo,
    pub journeyman_price: Kpo,
}

/// Valeur d'équipe : une somme recalculée, jamais une accumulation de deltas.
///
/// Trois règles s'y lisent en creux. Un joueur indisponible au prochain match
/// vaut zéro — mais il laisse une place à combler, donc un journalier. Le
/// Facteur Fans et la trésorerie n'entrent pas dans la TV. Les relances comptent
/// à leur prix de base, pas au montant déboursé : l'écart apparaîtra le jour où
/// une relance achetée en cours de saison coûtera double.
pub fn compute_team_value(inputs: &TeamValueInputs) -> Kpo {
    Kpo(players_value(&inputs.players)
        + journeymen_value(&inputs.players, inputs.journeyman_price)
        + staff_value(inputs)
        + rerolls_value(inputs))
}

/// Un joueur indisponible ne vaut rien : ni blessé absent, ni retraité, ni mort.
fn players_value(players: &[ValuedPlayer]) -> u32 {
    players
        .iter()
        .filter(|p| p.available_for_next_match)
        .map(|p| p.value_kpo.0)
        .sum()
}

fn available_count(players: &[ValuedPlayer]) -> u32 {
    players
        .iter()
        .filter(|p| p.available_for_next_match)
        .count() as u32
}

/// Les places manquantes pour atteindre onze, au prix de la ligne journalier du
/// roster. Un effectif de douze disponibles n'en appelle aucun.
fn journeymen_value(players: &[ValuedPlayer], journeyman_price: Kpo) -> u32 {
    let missing = MATCH_SQUAD_SIZE.saturating_sub(available_count(players));
    missing * journeyman_price.0
}

/// Apothicaires, assistants et meneuses de ban. Le Facteur Fans est
/// volontairement absent : il n'entre pas dans la valeur d'équipe.
fn staff_value(inputs: &TeamValueInputs) -> u32 {
    inputs.apothecaries.0 as u32 * inputs.apothecary_price.0
        + inputs.assistants.0 as u32 * inputs.assistant_price.0
        + inputs.cheerleaders.0 as u32 * inputs.cheerleader_price.0
}

fn rerolls_value(inputs: &TeamValueInputs) -> u32 {
    inputs.rerolls.0 as u32 * inputs.reroll_price.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(value: u32, available: bool) -> ValuedPlayer {
        ValuedPlayer {
            value_kpo: Kpo(value),
            available_for_next_match: available,
        }
    }

    /// Onze joueurs disponibles, aucun staff, aucune relance : la TV est la
    /// somme nue de l'effectif, sans journalier.
    fn inputs(players: Vec<ValuedPlayer>) -> TeamValueInputs {
        TeamValueInputs {
            players,
            rerolls: RerollCount(0),
            reroll_price: Kpo(60),
            apothecaries: ApothecaryCount(0),
            apothecary_price: Kpo(50),
            assistants: AssistantCount(0),
            assistant_price: Kpo(10),
            cheerleaders: CheerleaderCount(0),
            cheerleader_price: Kpo(10),
            journeyman_price: Kpo(50),
        }
    }

    fn squad(n: u32, value: u32) -> Vec<ValuedPlayer> {
        (0..n).map(|_| player(value, true)).collect()
    }

    #[test]
    fn effectif_complet_sans_indisponible_est_une_somme_simple() {
        let tv = compute_team_value(&inputs(squad(11, 50)));
        assert_eq!(tv, Kpo(550));
    }

    #[test]
    fn un_joueur_absent_au_prochain_match_est_exclu_et_remplace_par_un_journalier() {
        let mut players = squad(10, 50);
        players.push(player(90, false)); // blessé : 90 kPo qui ne comptent pas

        let tv = compute_team_value(&inputs(players));

        // 10 × 50 disponibles + 1 journalier à 50 — et surtout pas les 90 kPo
        assert_eq!(tv, Kpo(550));
    }

    #[test]
    fn un_joueur_retraite_est_traite_comme_un_absent() {
        let mut players = squad(10, 50);
        players.push(player(120, false));
        assert_eq!(compute_team_value(&inputs(players)), Kpo(550));
    }

    #[test]
    fn neuf_disponibles_appellent_deux_journaliers() {
        let tv = compute_team_value(&inputs(squad(9, 50)));
        assert_eq!(tv, Kpo(9 * 50 + 2 * 50));
    }

    #[test]
    fn quatorze_disponibles_n_appellent_aucun_journalier() {
        let tv = compute_team_value(&inputs(squad(14, 50)));
        assert_eq!(tv, Kpo(700));
    }

    /// Le Facteur Fans n'est pas un paramètre du calcul : il ne peut donc pas
    /// entrer dans le total, contrairement à ce que faisait l'incrémental, qui
    /// ajoutait son coût à `team_value` comme n'importe quel staff.
    #[test]
    fn le_facteur_fans_n_entre_pas_dans_le_total() {
        let mut i = inputs(squad(11, 50));
        i.apothecaries = ApothecaryCount(1);
        i.assistants = AssistantCount(2);
        i.cheerleaders = CheerleaderCount(3);

        let tv = compute_team_value(&i);

        // 550 + 50 + 2×10 + 3×10 = 650 — aucune place pour un Facteur Fans
        assert_eq!(tv, Kpo(650));
    }

    #[test]
    fn relances_et_staff_sont_comptes_au_prix_de_base() {
        let mut i = inputs(squad(11, 50));
        i.rerolls = RerollCount(2);
        i.apothecaries = ApothecaryCount(1);

        // 550 + 2×60 + 50 — le prix de base, quel qu'ait été le montant payé
        assert_eq!(compute_team_value(&i), Kpo(720));
    }
}
