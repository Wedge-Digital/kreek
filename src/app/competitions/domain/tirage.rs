//! Le tirage des rencontres d'une journée.
//!
//! Remplace `generate_round_pairings`, qui poursuivait le mauvais objectif :
//! il prenait gloutonnement les paires inédites dans l'ordre des indices, et
//! préférait donc **laisser des équipes sans match** plutôt que de programmer
//! une revanche. Mesuré à 54-58 % des tirages en milieu de saison, et prouvé
//! sur quatre équipes dont trois paires sont jouées — il n'en appariait qu'une.
//!
//! # Les objectifs, dans l'ordre
//!
//! | Rang | Règle | Nature |
//! |---|---|---|
//! | 1 | R10 — jamais deux équipes d'un même coach | contrainte dure |
//! | 2 | R8.1 — apparier le plus d'équipes possible | objectif |
//! | 3 | R8.2 — rejouer le moins, et le plus anciennement | objectif |
//! | 4 | R9 — exempter une équipe qui ne l'a jamais été | préférence |
//! | 5 | R17 — départager au sort | arbitrage |
//!
//! Seul le rang 1 refuse ; les autres cèdent dans l'ordre. **Une revanche est
//! préférable à une équipe qui rentre chez elle sans avoir joué.**
//!
//! # Programmation dynamique sur masque de bits
//!
//! La spec annonçait « énumération avec élagage ». Vérifié : à vingt équipes
//! c'est 19!! ≈ 6,5 × 10⁸ appariements complets, et le pire cas — aucune paire
//! interdite, historique vide — est justement celui où l'élagage n'élague rien.
//!
//! La DP visite 2ⁿ états, une vingtaine de transitions chacun. Elle est
//! **exacte**, et surtout plus facile à prouver juste qu'un branch-and-bound.
//!
//! Mesuré en profil `dev` — la production tourne en `release`, plus rapide d'un
//! ordre de grandeur :
//!
//! | Équipes | 8 | 12 | 14 | 16 | 18 | 20 |
//! |---|---|---|---|---|---|---|
//! | Durée | 0,8 ms | 1,5 ms | 6,4 ms | 34 ms | 130 ms | 557 ms |
//!
//! Le doublement à chaque paire d'équipes se lit dans le tableau. Une journée
//! de ligue amateur en compte huit à quatorze, où le tirage est instantané ;
//! `MAX_EQUIPES` borne le reste, et **refuse plutôt que de faire attendre**.
//!
//! # L'aléa reste au bord
//!
//! Choisir la meilleure combinaison est **déterministe** — c'est la DP, et
//! c'est ce qui rend R8 testable. Le générateur n'intervient qu'au moment de
//! reconstruire, pour départager les combinaisons restées à égalité (R17).

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::{MatchDayName, MatchDayPosition};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use rand::seq::IndexedRandom;
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Au-delà, la DP demanderait plus de mémoire que le problème ne vaut. Une
/// journée de ligue amateur en compte huit à quatorze.
pub const MAX_EQUIPES: usize = 20;

// ── Ce que le tirage sait de l'histoire ──────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NombreDeRencontres(pub u32);

/// Pourquoi le tirage a retenu cette paire — et ce que l'écran en dit.
///
/// Il vient du domaine, qui sait *pourquoi* il a concédé. Le laisser déduire à
/// la vue reviendrait à relire les journées depuis la présentation, ce que
/// « un view model transpose, il ne dérive pas » interdit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Historique {
    Inedite,
    Revanche {
        fois: NombreDeRencontres,
        derniere: MatchDayName,
    },
}

#[derive(Debug, Clone)]
struct DejaJouee {
    fois: NombreDeRencontres,
    position: MatchDayPosition,
    nom: MatchDayName,
}

/// Les rencontres déjà jouées de la saison, **comptées** par paire.
///
/// Un `HashSet` ne disait que « déjà jouée » : c'est ce choix de type, et non
/// une omission de logique, qui rendait la minimisation de R8.2 impossible.
#[derive(Debug, Clone, Default)]
pub struct RencontresJouees {
    par_paire: HashMap<(TeamId, TeamId), DejaJouee>,
}

impl RencontresJouees {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre une rencontre. Rappelée sur la même paire, elle incrémente le
    /// compte et retient la journée la plus récente.
    pub fn enregistrer(
        &mut self,
        a: &TeamId,
        b: &TeamId,
        position: MatchDayPosition,
        nom: MatchDayName,
    ) {
        let cle = paire(a, b);
        match self.par_paire.get_mut(&cle) {
            Some(deja) => deja.ajouter(position, nom),
            None => {
                self.par_paire.insert(
                    cle,
                    DejaJouee {
                        fois: NombreDeRencontres(1),
                        position,
                        nom,
                    },
                );
            }
        }
    }

    pub fn historique(&self, a: &TeamId, b: &TeamId) -> Historique {
        match self.par_paire.get(&paire(a, b)) {
            None => Historique::Inedite,
            Some(deja) => Historique::Revanche {
                fois: deja.fois,
                derniere: deja.nom.clone(),
            },
        }
    }

    fn poids(&self, a: &TeamId, b: &TeamId) -> (u32, i64) {
        match self.par_paire.get(&paire(a, b)) {
            None => (0, 0),
            Some(deja) => (deja.fois.0, i64::from(deja.position.into_inner())),
        }
    }
}

impl DejaJouee {
    fn ajouter(&mut self, position: MatchDayPosition, nom: MatchDayName) {
        self.fois = NombreDeRencontres(self.fois.0 + 1);
        if position.into_inner() >= self.position.into_inner() {
            self.position = position;
            self.nom = nom;
        }
    }
}

/// La paire normalisée — le plus petit identifiant d'abord, pour que `A-B` et
/// `B-A` soient la même clé.
///
/// La comparaison passe par `to_string()` faute d'`Ord` sur `SUlid`. C'est
/// correct : l'encodage Crockford d'un ULID est monotone, donc l'ordre des
/// chaînes est celui des identifiants. Doter `SUlid` d'`Ord` serait plus propre
/// et touche le `shared_kernel` — hors du périmètre de cette carte.
pub fn paire(a: &TeamId, b: &TeamId) -> (TeamId, TeamId) {
    if a.to_string() <= b.to_string() {
        (*a, *b)
    } else {
        (*b, *a)
    }
}

// ── Entrée et sortie ─────────────────────────────────────────────────────────

/// Ce dont le tirage a besoin. Un type, pas six paramètres.
///
/// `interdites` est rempli par le use case : la relation équipe → coach vit
/// dans un autre BC. La règle R10 est métier, sa **matière** est inter-BC — le
/// domaine reçoit des paires interdites, il n'a pas à savoir qu'un coach existe.
#[derive(Debug, Clone, Default)]
pub struct DrawInput {
    pub equipes: Vec<TeamId>,
    pub historique: RencontresJouees,
    pub interdites: HashSet<(TeamId, TeamId)>,
    pub jamais_exemptees: HashSet<TeamId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedPairing {
    pub home: TeamId,
    pub away: TeamId,
    pub historique: Historique,
}

/// La proposition. **Rien n'est écrit** — c'est le use case de validation qui
/// persiste, après revérification (R22).
///
/// `ecartees` reste vide ici : les équipes désengagées depuis leur réponse
/// (R18) sont filtrées par le use case, qui seul connaît les inscriptions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DrawProposal {
    pub rencontres: Vec<ProposedPairing>,
    pub exemptee: Option<TeamId>,
    pub ecartees: Vec<TeamId>,
}

// ── Le coût d'une combinaison ────────────────────────────────────────────────

/// Les quatre objectifs, comparés dans l'ordre. La comparaison lexicographique
/// **est** la hiérarchie de R8/R9 : aucun rang ne peut compenser le précédent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cout {
    /// R8.1 — le nombre d'équipes laissées sans match.
    perdues: u32,
    /// R8.2 — le total des rencontres déjà jouées que la combinaison reprogramme.
    revanches: u32,
    /// R8.2 — la somme des positions des journées reprises. Plus petit = plus
    /// ancien, donc préférable. Une paire inédite ne pèse rien.
    anciennete: i64,
    /// R9 — le nombre d'exemptées qui l'ont déjà été.
    exemptions_repetees: u32,
}

impl Cout {
    const ZERO: Cout = Cout {
        perdues: 0,
        revanches: 0,
        anciennete: 0,
        exemptions_repetees: 0,
    };

    const INFINI: Cout = Cout {
        perdues: u32::MAX,
        revanches: u32::MAX,
        anciennete: i64::MAX,
        exemptions_repetees: u32::MAX,
    };

    fn plus(self, autre: Cout) -> Cout {
        if self == Cout::INFINI || autre == Cout::INFINI {
            return Cout::INFINI;
        }
        Cout {
            perdues: self.perdues + autre.perdues,
            revanches: self.revanches + autre.revanches,
            anciennete: self.anciennete + autre.anciennete,
            exemptions_repetees: self.exemptions_repetees + autre.exemptions_repetees,
        }
    }
}

// ── Le tirage ────────────────────────────────────────────────────────────────

/// Tire les rencontres d'une journée.
///
/// Déterministe jusqu'au départage : deux appels sur la même entrée rendent des
/// appariements également optimaux, mais pas forcément les mêmes (R17).
pub fn tirer(input: &DrawInput, rng: &mut impl Rng) -> Result<DrawProposal, DomainError> {
    let n = input.equipes.len();
    if n > MAX_EQUIPES {
        return Err(DomainError::TropDEquipesPourLeTirage { equipes: n });
    }
    if n < 2 {
        return Ok(DrawProposal::default());
    }

    let table = Table::batir(input);
    let dp = table.resoudre();
    let (paires, exemptees) = table.reconstruire(&dp, rng);

    Ok(DrawProposal {
        rencontres: table.rencontres(input, &paires),
        exemptee: exemptees.first().map(|&i| input.equipes[i]),
        ecartees: vec![],
    })
}

/// Les données du problème, indexées par position plutôt que par identifiant :
/// la DP manipule des `usize` et des masques, jamais des ULID.
struct Table {
    n: usize,
    autorisee: Vec<Vec<bool>>,
    poids: Vec<Vec<(u32, i64)>>,
    deja_exemptee: Vec<bool>,
}

impl Table {
    fn batir(input: &DrawInput) -> Table {
        let n = input.equipes.len();
        let mut table = Table {
            n,
            autorisee: vec![vec![false; n]; n],
            poids: vec![vec![(0, 0); n]; n],
            deja_exemptee: vec![false; n],
        };
        for i in 0..n {
            table.deja_exemptee[i] = !input.jamais_exemptees.contains(&input.equipes[i]);
            for j in 0..n {
                table.remplir(input, i, j);
            }
        }
        table
    }

    /// R10 en filtre préalable : une paire interdite n'entre jamais dans la DP,
    /// donc aucun objectif ne peut la rattraper.
    fn remplir(&mut self, input: &DrawInput, i: usize, j: usize) {
        if i == j {
            return;
        }
        let (a, b) = (&input.equipes[i], &input.equipes[j]);
        self.autorisee[i][j] = !input.interdites.contains(&paire(a, b));
        self.poids[i][j] = input.historique.poids(a, b);
    }

    /// `dp[masque]` = le meilleur coût pour résoudre exactement ces équipes,
    /// chacune appariée à l'intérieur du masque ou laissée de côté.
    fn resoudre(&self) -> Vec<Cout> {
        let mut dp = vec![Cout::INFINI; 1usize << self.n];
        dp[0] = Cout::ZERO;
        for masque in 1..(1usize << self.n) {
            let i = masque.trailing_zeros() as usize;
            let reste = masque & !(1usize << i);
            let mut meilleur = dp[reste].plus(self.cout_exemption(i));
            for j in self.partenaires(i, reste) {
                let suivant = reste & !(1usize << j);
                meilleur = meilleur.min(dp[suivant].plus(self.cout_paire(i, j)));
            }
            dp[masque] = meilleur;
        }
        dp
    }

    fn partenaires(&self, i: usize, reste: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.n).filter(move |&j| reste & (1usize << j) != 0 && self.autorisee[i][j])
    }

    fn cout_exemption(&self, i: usize) -> Cout {
        Cout {
            perdues: 1,
            exemptions_repetees: u32::from(self.deja_exemptee[i]),
            ..Cout::ZERO
        }
    }

    fn cout_paire(&self, i: usize, j: usize) -> Cout {
        let (fois, position) = self.poids[i][j];
        Cout {
            revanches: fois,
            anciennete: position,
            ..Cout::ZERO
        }
    }

    /// Redescend la table en choisissant **au sort** parmi les transitions qui
    /// atteignent l'optimum — c'est R17, et c'est le seul endroit où le
    /// générateur intervient.
    fn reconstruire(&self, dp: &[Cout], rng: &mut impl Rng) -> (Vec<(usize, usize)>, Vec<usize>) {
        let (mut paires, mut exemptees) = (Vec::new(), Vec::new());
        let mut masque = (1usize << self.n) - 1;
        while masque != 0 {
            let i = masque.trailing_zeros() as usize;
            let reste = masque & !(1usize << i);
            match self.choisir(dp, i, reste, rng) {
                Some(j) => {
                    paires.push((i, j));
                    masque = reste & !(1usize << j);
                }
                None => {
                    exemptees.push(i);
                    masque = reste;
                }
            }
        }
        (paires, exemptees)
    }

    /// Le partenaire retenu pour `i`, ou `None` s'il faut l'exempter.
    ///
    /// La liste des candidats n'est jamais vide : l'exemption est toujours une
    /// transition possible, et `dp` est le minimum sur exactement ces
    /// transitions. Le `flatten` dégrade donc un cas qui ne se produit pas — et
    /// s'il se produisait, il laisserait une équipe de côté, jamais une paire
    /// fausse.
    fn choisir(&self, dp: &[Cout], i: usize, reste: usize, rng: &mut impl Rng) -> Option<usize> {
        let optimum = dp[reste | (1usize << i)];
        let mut candidats: Vec<Option<usize>> = self
            .partenaires(i, reste)
            .filter(|&j| dp[reste & !(1usize << j)].plus(self.cout_paire(i, j)) == optimum)
            .map(Some)
            .collect();
        if dp[reste].plus(self.cout_exemption(i)) == optimum {
            candidats.push(None);
        }
        candidats.choose(rng).copied().flatten()
    }

    fn rencontres(&self, input: &DrawInput, paires: &[(usize, usize)]) -> Vec<ProposedPairing> {
        paires
            .iter()
            .map(|&(i, j)| ProposedPairing {
                home: input.equipes[i],
                away: input.equipes[j],
                historique: input
                    .historique
                    .historique(&input.equipes[i], &input.equipes[j]),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// Des équipes nommées par leur rang, pour que les assertions se lisent.
    /// Les ULID sont engendrés puis triés : `equipe(0)` est toujours la plus
    /// petite, ce qui rend les paires normalisées prévisibles.
    fn equipes(n: usize) -> Vec<TeamId> {
        let mut ids: Vec<TeamId> = (0..n).map(|_| TeamId::new()).collect();
        ids.sort_by_key(|id| id.to_string());
        ids
    }

    fn journee(position: i32) -> (MatchDayPosition, MatchDayName) {
        (
            MatchDayPosition::try_new(position).unwrap(),
            MatchDayName::try_new(format!("Journée {position}")).unwrap(),
        )
    }

    fn entree(equipes: Vec<TeamId>) -> DrawInput {
        DrawInput {
            equipes,
            ..DrawInput::default()
        }
    }

    fn jouee(input: &mut DrawInput, a: usize, b: usize, position: i32) {
        let (pos, nom) = journee(position);
        let (x, y) = (input.equipes[a], input.equipes[b]);
        input.historique.enregistrer(&x, &y, pos, nom);
    }

    fn graine(n: u64) -> StdRng {
        StdRng::seed_from_u64(n)
    }

    fn appariees(p: &DrawProposal) -> HashSet<(TeamId, TeamId)> {
        p.rencontres
            .iter()
            .map(|r| paire(&r.home, &r.away))
            .collect()
    }

    // ── R8.1 — apparier le plus d'équipes possible ───────────────────────────

    /// **Le test qui manquait.** Quatre équipes dont trois paires sont déjà
    /// jouées : `generate_round_pairings` n'en appariait qu'une, laissant deux
    /// équipes rentrer chez elles.
    ///
    /// Vérifié avant correction, sur la fonction en service :
    /// `obtenu : [("team-1", "team-4")]` — une rencontre au lieu de deux.
    #[test]
    fn quatre_equipes_trois_paires_jouees_donnent_deux_rencontres() {
        let mut input = entree(equipes(4));
        jouee(&mut input, 0, 1, 1);
        jouee(&mut input, 0, 2, 2);
        jouee(&mut input, 1, 2, 3);

        let p = tirer(&input, &mut graine(1)).unwrap();

        assert_eq!(p.rencontres.len(), 2, "obtenu : {:?}", p.rencontres);
        assert!(p.exemptee.is_none());
        let revanches = p
            .rencontres
            .iter()
            .filter(|r| r.historique != Historique::Inedite)
            .count();
        assert_eq!(revanches, 1, "une revanche est concédée, pas deux");
    }

    /// Une équipe non appariée coûte plus cher que n'importe quelle revanche :
    /// même toutes paires jouées, tout le monde joue.
    #[test]
    fn toutes_les_paires_jouees_apparie_quand_meme_tout_le_monde() {
        let mut input = entree(equipes(4));
        for (a, b) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
            jouee(&mut input, a, b, 1);
        }

        let p = tirer(&input, &mut graine(2)).unwrap();

        assert_eq!(p.rencontres.len(), 2);
    }

    // ── R8.2 — rejouer le moins, et le plus anciennement ─────────────────────

    /// À nombre de rencontres égal, la combinaison au moindre total de
    /// revanches gagne.
    ///
    /// Cinq paires sur six sont jouées, ce qui ne laisse que trois
    /// appariements complets possibles : `{0-1, 2-3}` coûte deux revanches,
    /// `{0-3, 1-2}` deux aussi, et `{0-2, 1-3}` une seule — `1-3` étant la
    /// seule paire inédite. Il faut fermer les autres portes pour que le
    /// critère se voie : à trois paires jouées, une combinaison à **zéro**
    /// revanche subsiste et le test ne prouverait rien.
    #[test]
    fn a_nombre_egal_la_combinaison_au_moindre_total_gagne() {
        let mut input = entree(equipes(4));
        for (a, b) in [(0, 1), (2, 3), (0, 2), (0, 3), (1, 2)] {
            jouee(&mut input, a, b, 1);
        }

        let p = tirer(&input, &mut graine(3)).unwrap();

        let attendu: HashSet<(TeamId, TeamId)> = HashSet::from([
            paire(&input.equipes[0], &input.equipes[2]),
            paire(&input.equipes[1], &input.equipes[3]),
        ]);
        assert_eq!(appariees(&p), attendu);
    }

    /// Entre deux revanches également coûteuses, la plus ancienne est préférée.
    /// 0-1 date de la journée 1, 0-2 de la journée 5 : c'est 0-1 qu'on reprend.
    #[test]
    fn entre_deux_revanches_la_plus_ancienne_est_preferee() {
        let mut input = entree(equipes(4));
        jouee(&mut input, 0, 1, 1);
        jouee(&mut input, 0, 2, 5);
        jouee(&mut input, 0, 3, 5);
        jouee(&mut input, 1, 2, 5);
        jouee(&mut input, 1, 3, 5);
        jouee(&mut input, 2, 3, 5);

        let p = tirer(&input, &mut graine(4)).unwrap();

        assert!(
            appariees(&p).contains(&paire(&input.equipes[0], &input.equipes[1])),
            "la rencontre la plus ancienne devait être reprise : {:?}",
            p.rencontres
        );
    }

    /// Le motif de la revanche vient du domaine, pas de la vue.
    #[test]
    fn une_revanche_porte_son_compte_et_sa_derniere_journee() {
        let mut input = entree(equipes(2));
        jouee(&mut input, 0, 1, 2);
        jouee(&mut input, 0, 1, 7);

        let p = tirer(&input, &mut graine(5)).unwrap();

        assert_eq!(
            p.rencontres[0].historique,
            Historique::Revanche {
                fois: NombreDeRencontres(2),
                derniere: MatchDayName::try_new("Journée 7").unwrap(),
            }
        );
    }

    // ── R9 — l'exemption ne se répète pas ────────────────────────────────────

    #[test]
    fn l_exemptee_est_tiree_parmi_celles_qui_ne_l_ont_jamais_ete() {
        let mut input = entree(equipes(5));
        input.jamais_exemptees = HashSet::from([input.equipes[3]]);

        for graine_n in 0..20 {
            let p = tirer(&input, &mut graine(graine_n)).unwrap();
            assert_eq!(p.rencontres.len(), 2);
            assert_eq!(p.exemptee, Some(input.equipes[3]));
        }
    }

    /// R9 cède devant R8.2 : elle est au rang 4, pas au rang 3. Exempter la
    /// seule équipe jamais exemptée coûterait ici une revanche de plus.
    /// Trois équipes, seule la 0 n'a jamais été exemptée, et 1-2 est déjà
    /// jouée. L'exempter elle coûterait une revanche ; on exempte donc la 1 ou
    /// la 2, et R9 cède.
    #[test]
    fn r9_cede_devant_le_nombre_de_revanches() {
        let mut input = entree(equipes(3));
        input.jamais_exemptees = HashSet::from([input.equipes[0]]);
        jouee(&mut input, 1, 2, 1);

        let p = tirer(&input, &mut graine(6)).unwrap();

        assert_ne!(
            p.exemptee,
            Some(input.equipes[0]),
            "exempter la seule jamais exemptée aurait coûté une revanche"
        );
        assert_eq!(
            p.rencontres[0].historique,
            Historique::Inedite,
            "on préfère une rencontre inédite à l'exemption souhaitée"
        );
    }

    // ── R10 — deux équipes d'un même coach ne se rencontrent jamais ──────────

    /// Contrainte dure : elle tient **même au prix d'un match en moins**.
    #[test]
    fn une_paire_interdite_n_est_jamais_appariee_meme_a_ce_prix() {
        let mut input = entree(equipes(2));
        input.interdites = HashSet::from([paire(&input.equipes[0], &input.equipes[1])]);

        let p = tirer(&input, &mut graine(7)).unwrap();

        assert!(p.rencontres.is_empty(), "obtenu : {:?}", p.rencontres);
    }

    #[test]
    fn une_paire_interdite_est_contournee_quand_c_est_possible() {
        let mut input = entree(equipes(4));
        input.interdites = HashSet::from([paire(&input.equipes[0], &input.equipes[1])]);

        for graine_n in 0..20 {
            let p = tirer(&input, &mut graine(graine_n)).unwrap();
            assert_eq!(p.rencontres.len(), 2);
            assert!(!appariees(&p).contains(&paire(&input.equipes[0], &input.equipes[1])));
        }
    }

    // ── R17 — c'est un vrai tirage ───────────────────────────────────────────

    /// Deux graines différentes donnent des appariements différents dès que
    /// plusieurs combinaisons sont également bonnes. Sans cette propriété, le
    /// bouton « Retirer au sort » ne ferait rien.
    #[test]
    fn deux_graines_donnent_des_appariements_differents() {
        let input = entree(equipes(6));

        let vus: HashSet<Vec<(TeamId, TeamId)>> = (0..30)
            .map(|g| {
                let p = tirer(&input, &mut graine(g)).unwrap();
                let mut paires: Vec<_> = appariees(&p).into_iter().collect();
                paires.sort_by_key(|(a, b)| (a.to_string(), b.to_string()));
                paires
            })
            .collect();

        assert!(
            vus.len() > 1,
            "trente tirages ont donné le même appariement"
        );
    }

    /// L'aléa départage, il ne dégrade pas : tous les tirages restent optimaux.
    #[test]
    fn l_alea_ne_degrade_jamais_le_resultat() {
        let mut input = entree(equipes(6));
        jouee(&mut input, 0, 1, 1);
        jouee(&mut input, 2, 3, 1);

        for graine_n in 0..30 {
            let p = tirer(&input, &mut graine(graine_n)).unwrap();
            assert_eq!(p.rencontres.len(), 3);
            let revanches = p
                .rencontres
                .iter()
                .filter(|r| r.historique != Historique::Inedite)
                .count();
            assert_eq!(revanches, 0, "graine {graine_n} : {:?}", p.rencontres);
        }
    }

    // ── Les bords ────────────────────────────────────────────────────────────

    #[test]
    fn moins_de_deux_equipes_ne_donne_aucune_rencontre() {
        assert!(tirer(&entree(vec![]), &mut graine(8))
            .unwrap()
            .rencontres
            .is_empty());
        assert!(tirer(&entree(equipes(1)), &mut graine(8))
            .unwrap()
            .rencontres
            .is_empty());
    }

    #[test]
    fn un_effectif_impair_exempte_exactement_une_equipe() {
        let input = entree(equipes(7));
        let p = tirer(&input, &mut graine(9)).unwrap();

        assert_eq!(p.rencontres.len(), 3);
        assert!(p.exemptee.is_some());
    }

    #[test]
    fn au_dela_du_plafond_le_tirage_refuse_et_le_dit() {
        let input = entree(equipes(MAX_EQUIPES + 1));

        let refus = tirer(&input, &mut graine(10));

        assert_eq!(
            refus,
            Err(DomainError::TropDEquipesPourLeTirage {
                equipes: MAX_EQUIPES + 1
            })
        );
    }

    /// `ecartees` appartient au use case, qui seul connaît les inscriptions.
    #[test]
    fn le_tirage_n_ecarte_personne_lui_meme() {
        let p = tirer(&entree(equipes(4)), &mut graine(11)).unwrap();
        assert!(p.ecartees.is_empty());
    }
}
