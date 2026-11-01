use crate::app::competitions::domain::draw::pair;
use crate::app::competitions::domain::error::DomainError;
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

    /// R9 — une journée de match, sans rencontres.
    pub fn can_receive_draw(&self) -> Result<(), DomainError> {
        if self.is_rest() {
            return Err(DomainError::RestDayCannotBeDrawn);
        }
        if !self.pairings.is_empty() {
            return Err(DomainError::RoundAlreadyPaired);
        }
        Ok(())
    }

    /// R3, R4, R11 — accepte le tirage et y ajoute ses rencontres, ou refuse
    /// sans rien changer.
    ///
    /// **Tout est vérifié avant que rien ne soit poussé** : un refus à la
    /// troisième rencontre ne laisse pas les deux premières dans la journée.
    pub fn accept_draw(
        &mut self,
        pairings: Vec<Pairing>,
        exempted: &[TeamId],
        rules: &DrawRules,
    ) -> Result<(), DomainError> {
        self.can_receive_draw()?;
        let mut seen = HashSet::new();
        for pairing in &pairings {
            Self::check_team(&pairing.home_team_id, rules, &mut seen)?;
            Self::check_team(&pairing.away_team_id, rules, &mut seen)?;
            Self::check_pair(pairing, rules)?;
        }
        Self::check_exempted(exempted, &seen)?;
        self.pairings.extend(pairings);
        Ok(())
    }

    /// R11 — inscrite, et tirée une seule fois. `seen` attrape aussi une
    /// équipe opposée à elle-même.
    fn check_team(
        team: &TeamId,
        rules: &DrawRules,
        seen: &mut HashSet<TeamId>,
    ) -> Result<(), DomainError> {
        if !rules.enrolled.contains(team) {
            return Err(DomainError::TeamNotEnrolled { team: *team });
        }
        if !seen.insert(*team) {
            return Err(DomainError::TeamDrawnTwice { team: *team });
        }
        Ok(())
    }

    /// R4 puis R3 — pas deux équipes d'un même coach, et une poule en commun.
    fn check_pair(pairing: &Pairing, rules: &DrawRules) -> Result<(), DomainError> {
        let (home, away) = (pairing.home_team_id, pairing.away_team_id);
        if rules.forbidden.contains(&pair(&home, &away)) {
            return Err(DomainError::SameCoachPairing { home, away });
        }
        let same_pool = rules
            .pools
            .iter()
            .any(|pool| pool.contains(&home) && pool.contains(&away));
        if !same_pool {
            return Err(DomainError::PairingAcrossPools { home, away });
        }
        Ok(())
    }

    /// R11 — les exemptées se reposent : aucune ne figure dans une rencontre.
    /// Une par poule impaire, d'où une liste.
    fn check_exempted(exempted: &[TeamId], seen: &HashSet<TeamId>) -> Result<(), DomainError> {
        match exempted.iter().find(|team| seen.contains(team)) {
            Some(team) => Err(DomainError::ExemptedTeamAlsoPaired { team: *team }),
            None => Ok(()),
        }
    }
}

/// Ce que la journée ne sait pas d'elle-même, fourni par le use case.
///
/// **Les poules sont des ensembles, sans identifiant** : la poule de repli
/// « Toutes les équipes » porte l'identifiant `"default"`, qui n'est pas un
/// `RankingGroupId` valide, et R3 n'a besoin que de savoir si deux équipes
/// partagent un ensemble.
#[derive(Debug, Clone, Default)]
pub struct DrawRules {
    pub enrolled: HashSet<TeamId>,
    /// Même coach — clé normalisée par [`pair`].
    pub forbidden: HashSet<(TeamId, TeamId)>,
    pub pools: Vec<HashSet<TeamId>>,
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

#[cfg(test)]
mod tests {
    use super::*;

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

    // ── Le tirage d'une journée (épic E18) ───────────────────────────────────

    /// Quatre équipes inscrites, toutes dans une seule poule.
    fn draw_setup() -> (Vec<TeamId>, DrawRules) {
        let teams: Vec<TeamId> = (0..4).map(|_| equipe()).collect();
        let all: HashSet<TeamId> = teams.iter().copied().collect();
        let rules = DrawRules {
            enrolled: all.clone(),
            forbidden: HashSet::new(),
            pools: vec![all],
        };
        (teams, rules)
    }

    fn refusal(pairings: Vec<Pairing>, exempted: &[TeamId], rules: &DrawRules) -> DomainError {
        let mut day = journee_avec(vec![]);
        let err = day.accept_draw(pairings, exempted, rules).unwrap_err();
        assert!(day.pairings.is_empty(), "un refus a modifié la journée");
        err
    }

    #[test]
    fn a_rest_day_cannot_receive_a_draw() {
        let mut day = journee_avec(vec![]);
        day.day_type = MatchDayType::Rest;
        assert_eq!(
            day.can_receive_draw(),
            Err(DomainError::RestDayCannotBeDrawn)
        );
    }

    #[test]
    fn a_paired_round_cannot_receive_a_draw() {
        let day = journee_avec(vec![rencontre(&equipe(), &equipe())]);
        assert_eq!(day.can_receive_draw(), Err(DomainError::RoundAlreadyPaired));
    }

    #[test]
    fn accept_draw_refuses_a_paired_round_without_touching_it() {
        let (t, rules) = draw_setup();
        let existing = rencontre(&t[0], &t[1]);
        let mut day = journee_avec(vec![existing.clone()]);
        let err = day.accept_draw(vec![rencontre(&t[2], &t[3])], &[], &rules);
        assert_eq!(err, Err(DomainError::RoundAlreadyPaired));
        assert_eq!(day.pairings, vec![existing]);
    }

    #[test]
    fn accept_draw_refuses_a_team_no_longer_enrolled() {
        let (t, mut rules) = draw_setup();
        rules.enrolled.remove(&t[3]);
        let err = refusal(
            vec![rencontre(&t[0], &t[1]), rencontre(&t[2], &t[3])],
            &[],
            &rules,
        );
        assert_eq!(err, DomainError::TeamNotEnrolled { team: t[3] });
    }

    #[test]
    fn accept_draw_refuses_a_team_drawn_twice() {
        let (t, rules) = draw_setup();
        let err = refusal(
            vec![rencontre(&t[0], &t[1]), rencontre(&t[1], &t[2])],
            &[],
            &rules,
        );
        assert_eq!(err, DomainError::TeamDrawnTwice { team: t[1] });
    }

    #[test]
    fn accept_draw_refuses_a_team_against_itself() {
        let (t, rules) = draw_setup();
        let err = refusal(vec![rencontre(&t[0], &t[0])], &[], &rules);
        assert_eq!(err, DomainError::TeamDrawnTwice { team: t[0] });
    }

    #[test]
    fn accept_draw_refuses_two_teams_of_the_same_coach() {
        let (t, mut rules) = draw_setup();
        rules.forbidden.insert(pair(&t[3], &t[2]));
        let err = refusal(
            vec![rencontre(&t[0], &t[1]), rencontre(&t[2], &t[3])],
            &[],
            &rules,
        );
        assert_eq!(
            err,
            DomainError::SameCoachPairing {
                home: t[2],
                away: t[3]
            }
        );
    }

    #[test]
    fn accept_draw_refuses_a_pairing_across_pools() {
        let (t, mut rules) = draw_setup();
        rules.pools = vec![
            [t[0], t[1]].into_iter().collect(),
            [t[2], t[3]].into_iter().collect(),
        ];
        let err = refusal(vec![rencontre(&t[0], &t[2])], &[], &rules);
        assert_eq!(
            err,
            DomainError::PairingAcrossPools {
                home: t[0],
                away: t[2]
            }
        );
    }

    #[test]
    fn accept_draw_refuses_an_exempted_team_also_paired() {
        let (t, rules) = draw_setup();
        let err = refusal(vec![rencontre(&t[0], &t[1])], &[t[1]], &rules);
        assert_eq!(err, DomainError::ExemptedTeamAlsoPaired { team: t[1] });
    }

    #[test]
    fn accept_draw_adds_every_pairing() {
        let (t, rules) = draw_setup();
        let drawn = vec![rencontre(&t[0], &t[1]), rencontre(&t[2], &t[3])];
        let mut day = journee_avec(vec![]);
        day.accept_draw(drawn.clone(), &[], &rules).unwrap();
        assert_eq!(day.pairings, drawn);
    }

    /// Deux poules impaires, deux exemptées : la journée les accepte ensemble.
    #[test]
    fn accept_draw_accepts_one_exempted_team_per_pool() {
        let teams: Vec<TeamId> = (0..6).map(|_| equipe()).collect();
        let rules = DrawRules {
            enrolled: teams.iter().copied().collect(),
            forbidden: HashSet::new(),
            pools: vec![
                teams[..3].iter().copied().collect(),
                teams[3..].iter().copied().collect(),
            ],
        };
        let drawn = vec![
            rencontre(&teams[0], &teams[1]),
            rencontre(&teams[3], &teams[4]),
        ];
        let mut day = journee_avec(vec![]);
        day.accept_draw(drawn.clone(), &[teams[2], teams[5]], &rules)
            .unwrap();
        assert_eq!(day.pairings, drawn);
    }
}
