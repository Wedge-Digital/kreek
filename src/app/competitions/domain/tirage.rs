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
//! # Un parcours avec retour sur trace, pas une table
//!
//! Deux voies ont été essayées. **La programmation dynamique sur masque de
//! bits** — une case par sous-ensemble d'équipes — est exacte et au temps
//! prévisible, mais elle garde 2ⁿ cases en mémoire : 24 Mo à vingt équipes,
//! 1,5 Go à vingt-six. C'est la **mémoire** qui bornait, pas le temps, et le
//! plafond tombait sous la taille réelle des ligues — la base de développement
//! porte une saison à vingt-deux équipes, et trois cent quarante-deux saisons
//! sans poule, dont toutes les équipes partent en un seul appel.
//!
//! Ce qui est écrit ici raisonne comme un organisateur devant sa feuille :
//!
//! 1. chaque équipe a un **univers** — les adversaires que rien n'interdit ;
//! 2. on traite d'abord **l'équipe la plus contrainte**, celle qui a le moins
//!    de choix : si une impasse existe, autant la rencontrer tout de suite ;
//! 3. on essaie ses adversaires **du moins cher au plus cher** — inédit
//!    d'abord, puis la revanche la plus ancienne ;
//! 4. dès qu'une branche coûte déjà plus que la meilleure solution connue, on
//!    l'abandonne ;
//! 5. si l'on retombe sur la borne inférieure du problème, on s'arrête : rien
//!    ne peut faire mieux.
//!
//! La mémoire est en O(n²) — les univers — au lieu de 2ⁿ. **Il n'y a plus de
//! plafond.** Le prix est un pire cas non borné, tenu par `BUDGET_NOEUDS`.
//!
//! # Ce que ça donne, mesuré
//!
//! Profil `dev` ; la production tourne en `release`, environ dix fois plus vite.
//! L'historique est la proportion de paires déjà jouées — 90 % est une fin de
//! saison où presque tout le monde s'est rencontré.
//!
//! | Historique | 14 éq. | 20 éq. | 24 éq. | 26 éq. | 31 éq. |
//! |---|---|---|---|---|---|
//! | 30 % | 1,8 ms | 0,9 ms | 1,2 ms | 0,9 ms | 1,4 ms |
//! | 50 % | 0,5 ms | 1,0 ms | 1,2 ms | 0,9 ms | 1,4 ms |
//! | 70 % | 1,7 ms | 0,9 ms | 1,5 ms | 18 ms | 1,4 ms |
//! | 90 % | 2,8 ms | 2,0 ms | 1,24 s | 1,00 s | 2,0 s ✂ |
//!
//! ✂ : le budget a coupé. **Le tirage rendu restait maximal** — quinze
//! rencontres sur trente et une équipes, soit tout ce qui est possible ; seule
//! l'optimalité du départage des revanches n'était pas prouvée.
//!
//! **Décupler le budget ne referme pas ce cas** : vingt millions de nœuds
//! coûtent vingt secondes au lieu de deux, pour le même appariement. Le budget
//! borne l'attente ; ce n'est pas un bouton « chercher plus fort ».
//!
//! # L'aléa reste au bord
//!
//! Choisir la meilleure combinaison est **déterministe** — c'est la DP, et
//! c'est ce qui rend R8 testable. Le générateur n'intervient qu'au moment de
//! reconstruire, pour départager les combinaisons restées à égalité (R17).

use crate::app::competitions::domain::match_day::{MatchDayName, MatchDayPosition};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Le pire cas d'un retour sur trace n'est pas borné. Plutôt que de faire
/// attendre indéfiniment, on arrête l'exploration au bout de ce nombre de
/// nœuds et l'on rend la meilleure solution trouvée — `DrawProposal` dit alors
/// que l'optimum n'est **pas prouvé**, plutôt que de le laisser croire.
const BUDGET_NOEUDS: u32 = 2_000_000;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawProposal {
    pub rencontres: Vec<ProposedPairing>,
    pub exemptee: Option<TeamId>,
    pub ecartees: Vec<TeamId>,
    /// Faux quand le budget de nœuds a coupé la recherche : la proposition est
    /// un appariement valide, mais rien ne dit qu'il soit le meilleur. L'écran
    /// le signale plutôt que de laisser croire à un optimum.
    pub optimum_prouve: bool,
}

impl Default for DrawProposal {
    fn default() -> Self {
        Self {
            rencontres: vec![],
            exemptee: None,
            ecartees: vec![],
            optimum_prouve: true,
        }
    }
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
pub fn tirer(input: &DrawInput, rng: &mut impl Rng) -> DrawProposal {
    if input.equipes.len() < 2 {
        return DrawProposal::default();
    }
    let mut recherche = Recherche::batir(input);
    let mut libres = vec![true; input.equipes.len()];
    let mut courant = Solution::default();
    recherche.explorer(&mut libres, &mut courant, Cout::ZERO, rng);
    recherche.proposition(input)
}

#[derive(Debug, Clone, Default)]
struct Solution {
    paires: Vec<(usize, usize)>,
    exemptees: Vec<usize>,
}

/// L'état de la recherche. Les équipes y sont des indices : on manipule des
/// `usize`, jamais des ULID.
struct Recherche {
    n: usize,
    /// R10 en filtre préalable : une paire interdite n'entre jamais dans un
    /// univers, donc aucun objectif ne peut la rattraper.
    autorisee: Vec<Vec<bool>>,
    poids: Vec<Vec<(u32, i64)>>,
    deja_exemptee: Vec<bool>,
    /// Ce qu'aucune solution ne peut battre. L'atteindre arrête tout.
    plancher: Cout,
    budget: u32,
    budget_epuise: bool,
    meilleur_cout: Cout,
    meilleure: Solution,
}

impl Recherche {
    fn batir(input: &DrawInput) -> Recherche {
        let n = input.equipes.len();
        let mut recherche = Recherche {
            n,
            autorisee: vec![vec![false; n]; n],
            poids: vec![vec![(0, 0); n]; n],
            deja_exemptee: vec![false; n],
            plancher: Cout::ZERO,
            budget: BUDGET_NOEUDS,
            budget_epuise: false,
            meilleur_cout: Cout::INFINI,
            meilleure: Solution::default(),
        };
        for i in 0..n {
            recherche.deja_exemptee[i] = !input.jamais_exemptees.contains(&input.equipes[i]);
            for j in 0..n {
                recherche.remplir(input, i, j);
            }
        }
        recherche.plancher = recherche.plancher();
        recherche
    }

    fn remplir(&mut self, input: &DrawInput, i: usize, j: usize) {
        if i == j {
            return;
        }
        let (a, b) = (&input.equipes[i], &input.equipes[j]);
        self.autorisee[i][j] = !input.interdites.contains(&paire(a, b));
        self.poids[i][j] = input.historique.poids(a, b);
    }

    /// La borne inférieure du problème : la parité impose une exemption, et si
    /// toutes les équipes ont déjà été exemptées, R9 sera fatalement violée.
    /// Une solution qui l'atteint est optimale — inutile de chercher plus loin.
    fn plancher(&self) -> Cout {
        let perdues = (self.n % 2) as u32;
        let toutes_deja = self.deja_exemptee.iter().all(|&d| d);
        Cout {
            perdues,
            exemptions_repetees: u32::from(perdues == 1 && toutes_deja),
            ..Cout::ZERO
        }
    }

    /// Rend `true` quand il faut arrêter toute l'exploration — plancher atteint
    /// ou budget épuisé.
    fn explorer(
        &mut self,
        libres: &mut [bool],
        courant: &mut Solution,
        cout: Cout,
        rng: &mut impl Rng,
    ) -> bool {
        if self.budget == 0 {
            self.budget_epuise = true;
            return true;
        }
        self.budget -= 1;
        if cout >= self.meilleur_cout {
            return false;
        }
        let Some(i) = self.plus_contrainte(libres, rng) else {
            return self.enregistrer(courant, cout);
        };
        libres[i] = false;
        let arret = self.essayer(i, libres, courant, cout, rng);
        libres[i] = true;
        arret
    }

    fn essayer(
        &mut self,
        i: usize,
        libres: &mut [bool],
        courant: &mut Solution,
        cout: Cout,
        rng: &mut impl Rng,
    ) -> bool {
        for option in self.univers(i, libres, rng) {
            let arret = match option {
                Some(j) => self.essayer_paire(i, j, libres, courant, cout, rng),
                None => self.essayer_exemption(i, libres, courant, cout, rng),
            };
            if arret {
                return true;
            }
        }
        false
    }

    fn essayer_paire(
        &mut self,
        i: usize,
        j: usize,
        libres: &mut [bool],
        courant: &mut Solution,
        cout: Cout,
        rng: &mut impl Rng,
    ) -> bool {
        libres[j] = false;
        courant.paires.push((i, j));
        let arret = self.explorer(libres, courant, cout.plus(self.cout_paire(i, j)), rng);
        courant.paires.pop();
        libres[j] = true;
        arret
    }

    fn essayer_exemption(
        &mut self,
        i: usize,
        libres: &mut [bool],
        courant: &mut Solution,
        cout: Cout,
        rng: &mut impl Rng,
    ) -> bool {
        courant.exemptees.push(i);
        let arret = self.explorer(libres, courant, cout.plus(self.cout_exemption(i)), rng);
        courant.exemptees.pop();
        arret
    }

    fn enregistrer(&mut self, courant: &Solution, cout: Cout) -> bool {
        if cout < self.meilleur_cout {
            self.meilleur_cout = cout;
            self.meilleure = courant.clone();
        }
        self.meilleur_cout <= self.plancher
    }

    /// L'équipe libre au plus petit univers — *fail-first*. Si une impasse
    /// existe, on la rencontre tout de suite plutôt qu'après dix choix à
    /// défaire ; c'est ce qui fait tenir la recherche.
    fn plus_contrainte(&self, libres: &[bool], rng: &mut impl Rng) -> Option<usize> {
        let mut candidats: Vec<usize> = Vec::new();
        let mut plus_petit = usize::MAX;
        for i in (0..self.n).filter(|&i| libres[i]) {
            let taille = self.taille_univers(i, libres);
            if taille < plus_petit {
                plus_petit = taille;
                candidats.clear();
            }
            if taille == plus_petit {
                candidats.push(i);
            }
        }
        candidats.choose(rng).copied()
    }

    fn taille_univers(&self, i: usize, libres: &[bool]) -> usize {
        (0..self.n)
            .filter(|&j| j != i && libres[j] && self.autorisee[i][j])
            .count()
    }

    /// Les adversaires possibles de `i`, **du moins cher au plus cher**, puis
    /// l'exemption — qui coûte toujours davantage, R8.1 dominant.
    ///
    /// Le mélange précède un tri **stable** : l'ordre des coûts est respecté,
    /// et le sort ne départage que les égalités. C'est R17, et c'est aussi ce
    /// qui fait que la première descente est déjà une bonne solution.
    fn univers(&self, i: usize, libres: &[bool], rng: &mut impl Rng) -> Vec<Option<usize>> {
        let mut partenaires: Vec<usize> = (0..self.n)
            .filter(|&j| libres[j] && self.autorisee[i][j])
            .collect();
        partenaires.shuffle(rng);
        partenaires.sort_by_key(|&j| self.poids[i][j]);
        let mut options: Vec<Option<usize>> = partenaires.into_iter().map(Some).collect();
        options.push(None);
        options
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

    fn proposition(&self, input: &DrawInput) -> DrawProposal {
        DrawProposal {
            rencontres: self.rencontres(input),
            exemptee: self.meilleure.exemptees.first().map(|&i| input.equipes[i]),
            ecartees: vec![],
            optimum_prouve: !self.budget_epuise,
        }
    }

    fn rencontres(&self, input: &DrawInput) -> Vec<ProposedPairing> {
        self.meilleure
            .paires
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

        let p = tirer(&input, &mut graine(1));

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

        let p = tirer(&input, &mut graine(2));

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

        let p = tirer(&input, &mut graine(3));

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

        let p = tirer(&input, &mut graine(4));

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

        let p = tirer(&input, &mut graine(5));

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
            let p = tirer(&input, &mut graine(graine_n));
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

        let p = tirer(&input, &mut graine(6));

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

        let p = tirer(&input, &mut graine(7));

        assert!(p.rencontres.is_empty(), "obtenu : {:?}", p.rencontres);
    }

    #[test]
    fn une_paire_interdite_est_contournee_quand_c_est_possible() {
        let mut input = entree(equipes(4));
        input.interdites = HashSet::from([paire(&input.equipes[0], &input.equipes[1])]);

        for graine_n in 0..20 {
            let p = tirer(&input, &mut graine(graine_n));
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
                let p = tirer(&input, &mut graine(g));
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
            let p = tirer(&input, &mut graine(graine_n));
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
        assert!(tirer(&entree(vec![]), &mut graine(8)).rencontres.is_empty());
        assert!(tirer(&entree(equipes(1)), &mut graine(8))
            .rencontres
            .is_empty());
    }

    #[test]
    fn un_effectif_impair_exempte_exactement_une_equipe() {
        let input = entree(equipes(7));
        let p = tirer(&input, &mut graine(9));

        assert_eq!(p.rencontres.len(), 3);
        assert!(p.exemptee.is_some());
    }

    /// Il n'y a plus de plafond : la recherche tient en O(n²) de mémoire, et la
    /// base de développement porte déjà une saison à vingt-deux équipes.
    #[test]
    fn une_grande_ligue_est_appariee_sans_plafond() {
        for n in [22, 26, 31] {
            let p = tirer(&entree(equipes(n)), &mut graine(10));
            assert_eq!(p.rencontres.len(), n / 2, "{n} équipes");
            assert_eq!(p.exemptee.is_some(), n % 2 == 1);
            assert!(p.optimum_prouve, "{n} équipes : budget épuisé");
        }
    }

    /// `ecartees` appartient au use case, qui seul connaît les inscriptions.
    #[test]
    fn le_tirage_n_ecarte_personne_lui_meme() {
        let p = tirer(&entree(equipes(4)), &mut graine(11));
        assert!(p.ecartees.is_empty());
    }

    // ── Les six tests hérités de `generate_round_pairings` ───────────────────
    //
    // Ils portent sur des **propriétés** — nombre de rencontres, absence de
    // doublon, normalisation — et non sur des appariements nommés. C'est ce qui
    // rend leur reprise possible telle quelle : seuls les types changent, pas
    // une seule assertion. Ils n'ont jamais vu le cas qui casse, mais ils
    // décrivent ce qui ne doit pas régresser.

    #[test]
    fn quatre_equipes_sans_historique_donnent_deux_rencontres() {
        let p = tirer(&entree(equipes(4)), &mut graine(20));

        assert_eq!(p.rencontres.len(), 2);
        let mut vues: HashSet<TeamId> = HashSet::new();
        for r in &p.rencontres {
            assert!(vues.insert(r.home), "équipe appariée deux fois");
            assert!(vues.insert(r.away), "équipe appariée deux fois");
        }
    }

    #[test]
    fn la_journee_suivante_ne_rejoue_aucune_paire_de_la_premiere() {
        let mut input = entree(equipes(4));
        let premiere = tirer(&input, &mut graine(21));
        assert_eq!(premiere.rencontres.len(), 2);
        for r in &premiere.rencontres {
            let (a, b) = (r.home, r.away);
            let (pos, nom) = journee(1);
            input.historique.enregistrer(&a, &b, pos, nom);
        }

        let seconde = tirer(&input, &mut graine(22));

        assert_eq!(seconde.rencontres.len(), 2);
        let deja = appariees(&premiere);
        for r in &seconde.rencontres {
            assert!(!deja.contains(&paire(&r.home, &r.away)), "paire répétée");
        }
    }

    #[test]
    fn trois_journees_epuisent_les_six_paires_puis_le_cycle_reprend() {
        let mut input = entree(equipes(4));
        let mut toutes: HashSet<(TeamId, TeamId)> = HashSet::new();

        for numero in 1..=3 {
            let p = tirer(&input, &mut graine(30 + numero as u64));
            for r in &p.rencontres {
                toutes.insert(paire(&r.home, &r.away));
                let (pos, nom) = journee(numero);
                input.historique.enregistrer(&r.home, &r.away, pos, nom);
            }
        }

        assert_eq!(toutes.len(), 6, "les six paires devaient être épuisées");
        assert_eq!(tirer(&input, &mut graine(34)).rencontres.len(), 2);
    }

    #[test]
    fn trois_equipes_donnent_une_rencontre() {
        assert_eq!(
            tirer(&entree(equipes(3)), &mut graine(23)).rencontres.len(),
            1
        );
    }

    #[test]
    fn zero_ou_une_equipe_ne_donne_rien() {
        assert!(tirer(&entree(vec![]), &mut graine(24))
            .rencontres
            .is_empty());
        assert!(tirer(&entree(equipes(1)), &mut graine(24))
            .rencontres
            .is_empty());
    }

    #[test]
    fn la_normalisation_traite_ab_et_ba_comme_une_seule_paire() {
        let mut input = entree(equipes(4));
        let (pos, nom) = journee(1);
        let (a, b) = (input.equipes[1], input.equipes[0]);
        input.historique.enregistrer(&a, &b, pos, nom);

        let p = tirer(&input, &mut graine(25));

        assert!(
            !appariees(&p).contains(&paire(&input.equipes[0], &input.equipes[1])),
            "la paire enregistrée dans l'autre sens n'a pas été reconnue"
        );
    }
}
