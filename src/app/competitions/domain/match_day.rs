use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::charset::TEXTE_SAISI;
use nutype::nutype;
use std::collections::HashSet;

/// Le nom d'une journée — son propre type, cf. [`SeasonName`].
#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 50, regex = TEXTE_SAISI),
    derive(
        Debug,
        Clone,
        Serialize,
        Deserialize,
        PartialEq,
        Eq,
        Hash,
        Display,
        AsRef
    )
)]
pub struct MatchDayName(String);

#[nutype(
    validate(greater_or_equal = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Display)
)]
pub struct MatchDayPosition(i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchDayType {
    FixedDate,
    TimeFrame,
    Rest,
}

impl MatchDayType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FixedDate => "fixed_date",
            Self::TimeFrame => "time_frame",
            Self::Rest => "rest",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "fixed_date" => Self::FixedDate,
            "rest" => Self::Rest,
            _ => Self::TimeFrame,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub id: PairingId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
}

#[derive(Debug, Clone)]
pub struct MatchDay {
    pub id: MatchId,
    pub season_id: SeasonId,
    pub name: MatchDayName,
    pub day_type: MatchDayType,
    pub date_start: Option<DateString>,
    pub date_end: Option<DateString>,
    pub position: MatchDayPosition,
    pub pairings: Vec<Pairing>,
}

impl MatchDay {
    pub fn is_rest(&self) -> bool {
        self.day_type == MatchDayType::Rest
    }

    /// L'appariement qui empêche `a` et `b` de se rencontrer cette journée —
    /// celui où **l'une des deux** est déjà engagée (carte 551).
    ///
    /// # Pourquoi l'appariement et non un booléen
    ///
    /// Le refus doit nommer l'équipe fautive et son adversaire : « Les Pillards
    /// affrontent déjà Lady's Ghosts ». Un `bool` obligerait l'appelant à
    /// reparcourir les appariements pour dire quoi que ce soit d'utile, alors
    /// que le parcours vient d'avoir lieu ici.
    ///
    /// # Les deux camps, et non le seul domicile
    ///
    /// Une équipe engagée **à l'extérieur** est tout aussi occupée. Ne regarder
    /// que `home_team_id` laisserait passer la moitié des cas — celle des
    /// déplacements — et le défaut ressemblerait à une donnée corrompue plutôt
    /// qu'à une règle incomplète.
    ///
    /// Le couple déjà programmé n'est pas un cas à part : si `a`-`b` existe,
    /// alors `a` est déjà engagée, et c'est cet appariement qui est rendu.
    pub fn engagement_existant(&self, a: &TeamId, b: &TeamId) -> Option<&Pairing> {
        self.pairings.iter().find(|p| p.engage(a) || p.engage(b))
    }

    /// Accueille une rencontre venue d'une autre journée (carte 557).
    ///
    /// Même règle que pour une rencontre nouvelle : si l'une des deux équipes
    /// joue déjà ici, le déplacement est refusé, et c'est **l'appariement
    /// bloquant** qui est rendu — l'appelant doit pouvoir dire lequel.
    pub fn accueillir(&mut self, pairing: Pairing) -> Result<(), Pairing> {
        if let Some(bloquant) =
            self.engagement_existant(&pairing.home_team_id, &pairing.away_team_id)
        {
            return Err(bloquant.clone());
        }
        self.pairings.push(pairing);
        Ok(())
    }

    /// Retire une rencontre de cette journée, et la rend pour qu'une autre
    /// l'accueille. `None` si elle n'y était pas.
    pub fn liberer(&mut self, pairing_id: &PairingId) -> Option<Pairing> {
        let position = self.pairings.iter().position(|p| &p.id == pairing_id)?;
        Some(self.pairings.remove(position))
    }
}

impl Pairing {
    /// `equipe` joue-t-elle cette rencontre, d'un camp ou de l'autre ?
    pub fn engage(&self, equipe: &TeamId) -> bool {
        self.home_team_id == *equipe || self.away_team_id == *equipe
    }

    /// L'adversaire de `equipe` dans cette rencontre, si elle y joue.
    pub fn adversaire_de(&self, equipe: &TeamId) -> Option<&TeamId> {
        if self.home_team_id == *equipe {
            Some(&self.away_team_id)
        } else if self.away_team_id == *equipe {
            Some(&self.home_team_id)
        } else {
            None
        }
    }
}

fn normalize_pair(a: &str, b: &str) -> (String, String) {
    if a < b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

pub fn generate_round_pairings(
    teams: &[String],
    already_played: &HashSet<(String, String)>,
) -> Vec<(String, String)> {
    if teams.len() < 2 {
        return vec![];
    }

    let all_pairs: Vec<(String, String)> = {
        let mut pairs = Vec::new();
        for i in 0..teams.len() {
            for j in (i + 1)..teams.len() {
                pairs.push(normalize_pair(&teams[i], &teams[j]));
            }
        }
        pairs
    };

    let mut available: Vec<&(String, String)> = all_pairs
        .iter()
        .filter(|p| !already_played.contains(*p))
        .collect();

    if available.is_empty() {
        available = all_pairs.iter().collect();
    }

    let mut result = Vec::new();
    let mut used: HashSet<&str> = HashSet::new();

    for pair in &available {
        if !used.contains(pair.0.as_str()) && !used.contains(pair.1.as_str()) {
            used.insert(&pair.0);
            used.insert(&pair.1);
            result.push((pair.0.clone(), pair.1.clone()));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn teams(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("team-{}", i + 1)).collect()
    }

    #[test]
    fn four_teams_no_played_generates_two_pairings() {
        let t = teams(4);
        let played = HashSet::new();
        let result = generate_round_pairings(&t, &played);
        assert_eq!(result.len(), 2);
        let mut used: HashSet<&str> = HashSet::new();
        for (a, b) in &result {
            assert!(used.insert(a));
            assert!(used.insert(b));
        }
    }

    #[test]
    fn four_teams_two_played_generates_different_pairings() {
        let t = teams(4);
        let mut played = HashSet::new();
        let first = generate_round_pairings(&t, &played);
        assert_eq!(first.len(), 2);

        for p in &first {
            played.insert(p.clone());
        }

        let second = generate_round_pairings(&t, &played);
        assert_eq!(second.len(), 2);
        for p in &second {
            assert!(!first.contains(p), "paire répétée : {:?}", p);
        }
    }

    #[test]
    fn four_teams_all_played_restarts_cycle() {
        let t = teams(4);
        let mut played = HashSet::new();

        for _ in 0..3 {
            let round = generate_round_pairings(&t, &played);
            for p in &round {
                played.insert(p.clone());
            }
        }

        assert_eq!(played.len(), 6);
        let next = generate_round_pairings(&t, &played);
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn three_teams_generates_one_pairing() {
        let t = teams(3);
        let played = HashSet::new();
        let result = generate_round_pairings(&t, &played);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn zero_or_one_team_generates_nothing() {
        assert!(generate_round_pairings(&[], &HashSet::new()).is_empty());
        assert!(generate_round_pairings(&teams(1), &HashSet::new()).is_empty());
    }

    #[test]
    fn normalization_treats_ab_and_ba_as_same() {
        let t = teams(4);
        let mut played = HashSet::new();
        played.insert(normalize_pair("team-2", "team-1"));

        let result = generate_round_pairings(&t, &played);
        for (a, b) in &result {
            let norm = normalize_pair(a, b);
            assert_ne!(norm, normalize_pair("team-1", "team-2"));
        }
    }

    // ── L'invariant de journée (carte 551) ───────────────────────────────────

    /// Un identifiant neuf. Pas de littéral « team-a » : `TeamId` est un ULID,
    /// et `try_new("team-a")` échoue en `InvalidLength`.
    fn equipe() -> TeamId {
        TeamId::new()
    }

    fn journee_avec(pairings: Vec<Pairing>) -> MatchDay {
        MatchDay {
            id: MatchId::new(),
            season_id: SeasonId::new(),
            name: MatchDayName::try_new("Journée 15".to_string()).unwrap(),
            day_type: MatchDayType::FixedDate,
            date_start: None,
            date_end: None,
            position: MatchDayPosition::try_new(14).unwrap(),
            pairings,
        }
    }

    fn rencontre(domicile: &TeamId, exterieur: &TeamId) -> Pairing {
        Pairing {
            id: PairingId::new(),
            home_team_id: domicile.clone(),
            away_team_id: exterieur.clone(),
        }
    }

    #[test]
    fn une_journee_vide_n_engage_personne() {
        let (a, b) = (equipe(), equipe());
        assert!(journee_avec(vec![]).engagement_existant(&a, &b).is_none());
    }

    #[test]
    fn une_equipe_engagee_a_domicile_bloque() {
        let (a, b, c) = (equipe(), equipe(), equipe());
        let journee = journee_avec(vec![rencontre(&a, &c)]);

        let bloquant = journee.engagement_existant(&a, &b).expect("a joue déjà");
        assert_eq!(bloquant.adversaire_de(&a), Some(&c));
    }

    /// **Le test qui compte.**
    ///
    /// Une implémentation ne regardant que `home_team_id` passerait tous les
    /// autres et manquerait celui-ci — c'est-à-dire la moitié des rencontres,
    /// celle des déplacements.
    #[test]
    fn une_equipe_engagee_a_l_exterieur_bloque_aussi() {
        let (a, b, c) = (equipe(), equipe(), equipe());
        let journee = journee_avec(vec![rencontre(&c, &a)]);

        let bloquant = journee.engagement_existant(&a, &b).expect("a joue déjà");
        assert_eq!(bloquant.adversaire_de(&a), Some(&c));
    }

    /// La seconde équipe compte autant que la première : `engagement_existant`
    /// n'est pas orienté.
    #[test]
    fn la_seconde_equipe_bloque_tout_autant() {
        let (a, b, c) = (equipe(), equipe(), equipe());
        let journee = journee_avec(vec![rencontre(&b, &c)]);

        assert!(journee.engagement_existant(&a, &b).is_some());
    }

    // ── Le déplacement d'une rencontre (carte 557) ────────────────────────────

    #[test]
    fn une_journee_libre_accueille_la_rencontre() {
        let (a, b) = (equipe(), equipe());
        let mut cible = journee_avec(vec![]);

        assert!(cible.accueillir(rencontre(&a, &b)).is_ok());
        assert_eq!(cible.pairings.len(), 1);
    }

    /// Le refus nomme l'appariement bloquant, comme pour un ajout : un
    /// déplacement ne contourne pas la règle de la carte 551.
    #[test]
    fn une_journee_ou_une_equipe_joue_deja_refuse_en_nommant_le_match() {
        let (a, b, c) = (equipe(), equipe(), equipe());
        let mut cible = journee_avec(vec![rencontre(&c, &b)]);

        let bloquant = cible
            .accueillir(rencontre(&a, &b))
            .expect_err("b joue déjà");
        assert_eq!(bloquant.adversaire_de(&b), Some(&c));
        assert_eq!(cible.pairings.len(), 1, "rien n'est ajouté sur un refus");
    }

    #[test]
    fn liberer_rend_la_rencontre_et_la_retire() {
        let (a, b) = (equipe(), equipe());
        let deplacee = rencontre(&a, &b);
        let mut depart = journee_avec(vec![deplacee.clone()]);

        assert_eq!(depart.liberer(&deplacee.id), Some(deplacee));
        assert!(depart.pairings.is_empty());
        assert_eq!(depart.liberer(&PairingId::new()), None);
    }

    /// Le couple déjà programmé n'est pas un cas à part — il tombe sous la même
    /// règle, et c'est pourquoi aucune vérification supplémentaire n'est écrite.
    #[test]
    fn le_meme_couple_est_deja_engage() {
        let (a, b) = (equipe(), equipe());
        let journee = journee_avec(vec![rencontre(&a, &b)]);

        assert!(journee.engagement_existant(&a, &b).is_some());
        // Et dans l'autre sens, l'appariement ayant pu retenir l'autre camp.
        assert!(journee.engagement_existant(&b, &a).is_some());
    }

    #[test]
    fn des_equipes_libres_ne_sont_pas_bloquees_par_les_autres_rencontres() {
        let (a, b) = (equipe(), equipe());
        let (c, d) = (equipe(), equipe());
        let journee = journee_avec(vec![rencontre(&c, &d)]);

        assert!(journee.engagement_existant(&a, &b).is_none());
    }

    #[test]
    fn adversaire_de_rend_none_pour_une_equipe_absente() {
        let (a, b, c) = (equipe(), equipe(), equipe());
        assert_eq!(rencontre(&a, &b).adversaire_de(&c), None);
    }
}
