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
    /// Ce joueur occupe-t-il la ligne marquée `is_journeyman` de son roster ?
    ///
    /// C'est la même ligne que celle dont le prix sert aux journaliers : la
    /// règle « Lineman a vil prix » n'en introduit pas une seconde définition.
    pub is_lineman: bool,
    /// Le prix du poste **au corpus d'aujourd'hui**, pas celui payé le jour du
    /// recrutement. C'est déjà le comportement des relances et du staff.
    pub base_cost: Kpo,
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
    /// La règle `LOW_COST_LINEMEN` du roster — « Lineman a vil prix ».
    ///
    /// Le domaine lit une règle, pas un identifiant de corpus : c'est
    /// l'adapter qui traduit l'uid.
    pub free_linemen: bool,
}

/// Valeur d'équipe : une somme recalculée, jamais une accumulation de deltas.
///
/// Trois règles s'y lisent en creux. Un joueur indisponible au prochain match
/// vaut zéro — mais il laisse une place à combler, donc un journalier. Le
/// Facteur Fans et la trésorerie n'entrent pas dans la TV. Les relances comptent
/// à leur prix de base, pas au montant déboursé : l'écart apparaîtra le jour où
/// une relance achetée en cours de saison coûtera double.
pub fn compute_team_value(inputs: &TeamValueInputs) -> Kpo {
    Kpo(players_value(&inputs.players, inputs.free_linemen)
        + journeymen_value(
            &inputs.players,
            inputs.journeyman_price,
            inputs.free_linemen,
        )
        + staff_value(inputs)
        + rerolls_value(inputs))
}

/// Un joueur indisponible ne vaut rien : ni blessé absent, ni retraité, ni mort.
///
/// Sous « Lineman a vil prix », un lineman ne compte que pour ce qu'il a gagné
/// au-delà de son prix — ses compétences et ses caractéristiques comptent
/// plein, son prix de base ne compte pas.
///
/// **La borne à zéro n'est pas décorative** : un commissaire peut avoir baissé
/// la valeur d'un joueur sous son prix de base par customisation. Un lineman
/// dans ce cas compte pour zéro, jamais en négatif.
fn players_value(players: &[ValuedPlayer], free_linemen: bool) -> u32 {
    players
        .iter()
        .filter(|p| p.available_for_next_match)
        .map(|p| {
            if free_linemen && p.is_lineman {
                p.value_kpo.0.saturating_sub(p.base_cost.0)
            } else {
                p.value_kpo.0
            }
        })
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
/// Les journaliers sont des linemen : sous la règle, ils ne coûtent rien non
/// plus. Les facturer alors que les vrais linemen sont gratuits reviendrait à
/// pénaliser un effectif incomplet plus qu'un effectif complet.
fn journeymen_value(players: &[ValuedPlayer], journeyman_price: Kpo, free_linemen: bool) -> u32 {
    if free_linemen {
        return 0;
    }
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
            is_lineman: false,
            base_cost: Kpo(0),
        }
    }

    /// Un lineman à son prix de base, sans amélioration.
    fn lineman(value: u32, base_cost: u32) -> ValuedPlayer {
        ValuedPlayer {
            value_kpo: Kpo(value),
            available_for_next_match: true,
            is_lineman: true,
            base_cost: Kpo(base_cost),
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
            free_linemen: false,
        }
    }

    fn inputs_vil_prix(players: Vec<ValuedPlayer>) -> TeamValueInputs {
        TeamValueInputs {
            free_linemen: true,
            ..inputs(players)
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

    // ── « Lineman a vil prix » ───────────────────────────────────────────────

    #[test]
    fn onze_linemen_nus_sous_la_regle_ne_valent_rien() {
        let effectif = (0..11).map(|_| lineman(50, 50)).collect();
        assert_eq!(compute_team_value(&inputs_vil_prix(effectif)), Kpo(0));
    }

    /// Le prix ne compte pas, les améliorations comptent plein : c'est toute la
    /// règle.
    #[test]
    fn un_lineman_ameliore_ne_compte_que_ses_ameliorations() {
        let mut effectif: Vec<ValuedPlayer> = (0..10).map(|_| lineman(50, 50)).collect();
        effectif.push(lineman(50 + 20 + 30, 50)); // deux compétences
        assert_eq!(compute_team_value(&inputs_vil_prix(effectif)), Kpo(50));
    }

    /// Un commissaire peut avoir baissé une valeur sous le prix de base. La
    /// valeur d'équipe ne devient pas négative pour autant.
    #[test]
    fn un_lineman_customise_sous_son_prix_compte_pour_zero() {
        let effectif = vec![lineman(30, 50)];
        // Onze places à combler moins une : les journaliers sont gratuits eux
        // aussi sous la règle, donc la TV est nulle et non négative.
        assert_eq!(compute_team_value(&inputs_vil_prix(effectif)), Kpo(0));
    }

    #[test]
    fn sous_la_regle_aucun_journalier_n_est_facture() {
        let effectif: Vec<ValuedPlayer> = (0..9).map(|_| lineman(50, 50)).collect();
        assert_eq!(compute_team_value(&inputs_vil_prix(effectif)), Kpo(0));
    }

    /// La règle ne vise que la ligne journalier du roster : un titulaire garde
    /// son prix, gratuité des linemen ou non.
    #[test]
    fn un_poste_non_lineman_garde_son_prix_sous_la_regle() {
        let mut effectif: Vec<ValuedPlayer> = (0..10).map(|_| lineman(50, 50)).collect();
        effectif.push(ValuedPlayer {
            value_kpo: Kpo(90),
            available_for_next_match: true,
            is_lineman: false,
            base_cost: Kpo(90),
        });
        assert_eq!(compute_team_value(&inputs_vil_prix(effectif)), Kpo(90));
    }

    /// Le témoin : sans la règle, rien ne change pour les mêmes joueurs.
    #[test]
    fn sans_la_regle_les_linemen_comptent_pour_leur_prix() {
        let effectif: Vec<ValuedPlayer> = (0..11).map(|_| lineman(50, 50)).collect();
        assert_eq!(compute_team_value(&inputs(effectif)), Kpo(550));
    }
}
