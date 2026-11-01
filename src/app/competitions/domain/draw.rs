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
//! | 1 | R4 — jamais deux équipes d'un même coach | contrainte dure |
//! | 2 | R5 — apparier le plus d'équipes possible | objectif |
//! | 3 | R5 — rejouer le moins, et le plus anciennement | objectif |
//! | 4 | R8 — exempter celle qui a le plus joué | préférence |
//! | 5 | départager au sort | arbitrage |
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
//! Choisir la meilleure combinaison est **déterministe**, et c'est ce qui
//! rend R5 et R8 testables. Le générateur n'intervient qu'au moment de
//! reconstruire, pour départager les combinaisons restées à égalité.

use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::domain::match_day::{MatchDayName, MatchDayPosition};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Le pire cas d'un retour sur trace n'est pas borné. Plutôt que de faire
/// attendre indéfiniment, on arrête l'exploration au bout de ce nombre de
/// nœuds et l'on rend la meilleure solution trouvée — `DrawProposal` dit alors
/// que l'optimum n'est **pas prouvé**, plutôt que de le laisser croire.
const NODE_BUDGET: u32 = 2_000_000;

// ── Ce que le tirage sait de l'histoire ──────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeetingCount(pub u32);

/// Le nombre d'appariements **programmés** d'une équipe sur la saison, joués ou
/// non. R8 s'en sert pour choisir l'exemptée.
///
/// Les appariements et non les rapports de match : ils vivent dans les tables du
/// BC, donc le compte se lit sans port, et la différence ne concerne que les
/// rencontres reportées.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct MatchesPlayed(pub u32);

/// Pourquoi le tirage a retenu cette paire — et ce que l'écran en dit.
///
/// Il vient du domaine, qui sait *pourquoi* il a concédé. Le laisser déduire à
/// la vue reviendrait à relire les journées depuis la présentation, ce que
/// « un view model transpose, il ne dérive pas » interdit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum History {
    FirstMeeting,
    Rematch {
        times: MeetingCount,
        last_round: MatchDayName,
    },
}

#[derive(Debug, Clone)]
struct PriorMeeting {
    times: MeetingCount,
    position: MatchDayPosition,
    name: MatchDayName,
}

/// Les rencontres déjà jouées de la saison, **comptées** par paire.
///
/// Un `HashSet` ne disait que « déjà jouée » : c'est ce choix de type, et non
/// une omission de logique, qui rendait la minimisation de R5 impossible.
#[derive(Debug, Clone, Default)]
pub struct PlayedMeetings {
    by_pair: HashMap<(TeamId, TeamId), PriorMeeting>,
}

impl PlayedMeetings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre une rencontre. Rappelée sur la même paire, elle incrémente le
    /// compte et retient la journée la plus récente.
    pub fn record(
        &mut self,
        a: &TeamId,
        b: &TeamId,
        position: MatchDayPosition,
        name: MatchDayName,
    ) {
        let key = pair(a, b);
        match self.by_pair.get_mut(&key) {
            Some(prior) => prior.add(position, name),
            None => {
                self.by_pair.insert(
                    key,
                    PriorMeeting {
                        times: MeetingCount(1),
                        position,
                        name,
                    },
                );
            }
        }
    }

    pub fn history(&self, a: &TeamId, b: &TeamId) -> History {
        match self.by_pair.get(&pair(a, b)) {
            None => History::FirstMeeting,
            Some(prior) => History::Rematch {
                times: prior.times,
                last_round: prior.name.clone(),
            },
        }
    }

    fn weight(&self, a: &TeamId, b: &TeamId) -> (u32, i64) {
        match self.by_pair.get(&pair(a, b)) {
            None => (0, 0),
            Some(prior) => (prior.times.0, i64::from(prior.position.into_inner())),
        }
    }
}

impl PriorMeeting {
    fn add(&mut self, position: MatchDayPosition, name: MatchDayName) {
        self.times = MeetingCount(self.times.0 + 1);
        if position.into_inner() >= self.position.into_inner() {
            self.position = position;
            self.name = name;
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
pub fn pair(a: &TeamId, b: &TeamId) -> (TeamId, TeamId) {
    if a.to_string() <= b.to_string() {
        (*a, *b)
    } else {
        (*b, *a)
    }
}

// ── Entrée et sortie ─────────────────────────────────────────────────────────

/// Ce dont le tirage a besoin. Un type, pas six paramètres.
///
/// `forbidden` est rempli par le use case : la relation équipe → coach vit
/// dans un autre BC. La règle R4 est métier, sa **matière** est inter-BC — le
/// domaine reçoit des paires interdites, il n'a pas à savoir qu'un coach existe.
#[derive(Debug, Clone, Default)]
pub struct DrawInput {
    pub teams: Vec<TeamId>,
    pub history: PlayedMeetings,
    pub forbidden: HashSet<(TeamId, TeamId)>,
    /// R8 — le nombre d'appariements de chaque équipe sur la saison. Une équipe
    /// absente de la table compte zéro : elle n'a rien joué, donc elle passe en
    /// dernier pour l'exemption.
    pub matches_played: HashMap<TeamId, MatchesPlayed>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedPairing {
    pub home: TeamId,
    pub away: TeamId,
    pub history: History,
}

/// La proposition. **Rien n'est écrit** — c'est le use case de validation qui
/// persiste, après revérification (R11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawProposal {
    pub pairings: Vec<ProposedPairing>,
    pub exempted: Option<TeamId>,
    /// Faux quand le budget de nœuds a coupé la recherche : la proposition est
    /// un appariement valide, mais rien ne dit qu'il soit le meilleur. L'écran
    /// ne l'affiche pas ; les tests s'en servent pour vérifier le budget.
    pub optimum_proven: bool,
}

impl Default for DrawProposal {
    fn default() -> Self {
        Self {
            pairings: vec![],
            exempted: None,
            optimum_proven: true,
        }
    }
}

// ── Le coût d'une combinaison ────────────────────────────────────────────────

/// Les quatre objectifs, comparés dans l'ordre. La comparaison lexicographique
/// **est** la hiérarchie de R5/R8 : aucun rang ne peut compenser le précédent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cost {
    /// R5 — le nombre d'équipes laissées sans match.
    unpaired: u32,
    /// R5 — le total des rencontres déjà jouées que la combinaison reprogramme.
    rematches: u32,
    /// R5 — la somme des positions des journées reprises. Plus petit = plus
    /// ancien, donc préférable. Une paire inédite ne pèse rien.
    staleness: i64,
    /// R8 — le retard de l'exemptée sur l'équipe la plus servie du groupe. Nul
    /// pour celle qui a le plus joué : l'exempter ne coûte rien.
    ///
    /// **Le retard, et non le compte brut.** `Cout` s'additionne le long de la
    /// recherche et se compare à un plancher ; une dimension portant un nombre
    /// de matchs absolu ferait dépendre l'élagage de l'avancement de la saison,
    /// et un plancher calculé à la journée 1 ne vaudrait plus rien à la 15.
    unfair_exemption: u32,
}

impl Cost {
    const ZERO: Cost = Cost {
        unpaired: 0,
        rematches: 0,
        staleness: 0,
        unfair_exemption: 0,
    };

    const INFINITE: Cost = Cost {
        unpaired: u32::MAX,
        rematches: u32::MAX,
        staleness: i64::MAX,
        unfair_exemption: u32::MAX,
    };

    fn plus(self, other: Cost) -> Cost {
        Cost {
            unpaired: self.unpaired + other.unpaired,
            rematches: self.rematches + other.rematches,
            staleness: self.staleness + other.staleness,
            unfair_exemption: self.unfair_exemption + other.unfair_exemption,
        }
    }
}

/// R8 — le retard de chaque équipe du groupe sur la plus servie.
///
/// Une équipe absente de `matchs_joues` compte zéro match : elle n'a rien joué,
/// donc son retard est maximal, donc elle passe en dernier pour l'exemption.
fn lags(input: &DrawInput) -> Vec<u32> {
    let played: Vec<u32> = input
        .teams
        .iter()
        .map(|t| input.matches_played.get(t).copied().unwrap_or_default().0)
        .collect();
    let most_served = played.iter().copied().max().unwrap_or(0);
    played.iter().map(|&j| most_served - j).collect()
}

// ── Le tirage ────────────────────────────────────────────────────────────────

/// Tire les rencontres d'une journée.
///
/// Déterministe jusqu'au départage : deux appels sur la même entrée rendent des
/// appariements également optimaux, mais pas forcément les mêmes.
pub fn draw(input: &DrawInput, rng: &mut impl Rng) -> DrawProposal {
    if input.teams.len() < 2 {
        // **Une équipe seule est exemptée, pas perdue.** Le raccourci doit dire
        // ce que la recherche aurait dit : elle range dans `exempted_teams` toute
        // équipe qu'elle n'apparie pas, y compris la dernière.
        //
        // Rendre `None` ici était sans conséquence tant que les deux appelants
        // garantissaient au moins deux équipes — `filter_enrolled_team_ids` pour
        // le Calendrier, `peut_tirer` pour le sondage. La réparation d'une
        // journée où il ne reste qu'un orphelin a fait apparaître le défaut :
        // elle annonçait « aucune exemptée » alors que quelqu'un restait sur le
        // banc (carte 518).
        return DrawProposal {
            exempted: input.teams.first().copied(),
            ..Default::default()
        };
    }
    let mut search = Search::build(input);
    let mut free = vec![true; input.teams.len()];
    let mut current = Solution::default();
    search.explore(&mut free, &mut current, Cost::ZERO, rng);
    search.proposal(input)
}

/// Tire chaque poule séparément (R3), dans l'ordre reçu.
///
/// R10 : une poule à moins de deux équipes rend une proposition sans
/// rencontre — son équipe unique exemptée, comme le fait déjà `draw` — et le
/// tirage n'est refusé que si **aucune** poule n'en a deux. Une poule creuse
/// n'empêche pas les autres de jouer.
pub fn draw_pools(
    inputs: &[DrawInput],
    rng: &mut impl Rng,
) -> Result<Vec<DrawProposal>, DomainError> {
    if inputs.iter().all(|input| input.teams.len() < 2) {
        return Err(DomainError::NotEnoughPresent);
    }
    Ok(inputs.iter().map(|input| draw(input, rng)).collect())
}

#[derive(Debug, Clone, Default)]
struct Solution {
    pairs: Vec<(usize, usize)>,
    exempted_teams: Vec<usize>,
}

/// L'état de la recherche. Les équipes y sont des indices : on manipule des
/// `usize`, jamais des ULID.
struct Search {
    n: usize,
    /// R4 en filtre préalable : une paire interdite n'entre jamais dans un
    /// univers, donc aucun objectif ne peut la rattraper.
    allowed: Vec<Vec<bool>>,
    weight: Vec<Vec<(u32, i64)>>,
    /// R8 — pour chaque équipe, son retard sur la plus servie **du groupe tiré**.
    ///
    /// Du groupe, et non de la saison : c'est ce qui garantit qu'un retard nul est
    /// toujours atteignable, donc que le plancher reste franchissable. Une poule
    /// qui joue moins qu'une autre n'a pas à en pâtir.
    lag: Vec<u32>,
    /// Ce qu'aucune solution ne peut battre. L'atteindre arrête tout.
    floor: Cost,
    budget: u32,
    budget_exhausted: bool,
    best_cost: Cost,
    best: Solution,
}

impl Search {
    fn build(input: &DrawInput) -> Search {
        let n = input.teams.len();
        let mut search = Search {
            n,
            allowed: vec![vec![false; n]; n],
            weight: vec![vec![(0, 0); n]; n],
            lag: lags(input),
            floor: Cost::ZERO,
            budget: NODE_BUDGET,
            budget_exhausted: false,
            best_cost: Cost::INFINITE,
            best: Solution::default(),
        };
        for i in 0..n {
            for j in 0..n {
                search.fill(input, i, j);
            }
        }
        search.floor = search.floor();
        search
    }

    fn fill(&mut self, input: &DrawInput, i: usize, j: usize) {
        if i == j {
            return;
        }
        let (a, b) = (&input.teams[i], &input.teams[j]);
        self.allowed[i][j] = !input.forbidden.contains(&pair(a, b));
        self.weight[i][j] = input.history.weight(a, b);
    }

    /// La borne inférieure du problème : la parité impose une exemption. Une
    /// solution qui l'atteint est optimale — inutile de chercher plus loin.
    ///
    /// **`exemption_injuste` y vaut zéro sans condition**, et c'est un progrès du
    /// nouveau critère : l'équipe la plus servie existe toujours, donc son retard
    /// est nul, donc exempter sans injustice est toujours possible. Le critère
    /// précédent avait un cas dégénéré — toutes les équipes déjà exemptées — qu'il
    /// fallait relever ici pour que la recherche puisse encore s'arrêter.
    fn floor(&self) -> Cost {
        Cost {
            unpaired: (self.n % 2) as u32,
            ..Cost::ZERO
        }
    }

    /// Rend `true` quand il faut arrêter toute l'exploration — plancher atteint
    /// ou budget épuisé.
    fn explore(
        &mut self,
        free: &mut [bool],
        current: &mut Solution,
        cost: Cost,
        rng: &mut impl Rng,
    ) -> bool {
        if self.budget == 0 {
            self.budget_exhausted = true;
            return true;
        }
        self.budget -= 1;
        if cost >= self.best_cost {
            return false;
        }
        let Some(i) = self.most_constrained(free, rng) else {
            return self.record(current, cost);
        };
        free[i] = false;
        let stop = self.attempt(i, free, current, cost, rng);
        free[i] = true;
        stop
    }

    fn attempt(
        &mut self,
        i: usize,
        free: &mut [bool],
        current: &mut Solution,
        cost: Cost,
        rng: &mut impl Rng,
    ) -> bool {
        for option in self.universe(i, free, rng) {
            let stop = match option {
                Some(j) => self.attempt_pair(i, j, free, current, cost, rng),
                None => self.attempt_exemption(i, free, current, cost, rng),
            };
            if stop {
                return true;
            }
        }
        false
    }

    fn attempt_pair(
        &mut self,
        i: usize,
        j: usize,
        free: &mut [bool],
        current: &mut Solution,
        cost: Cost,
        rng: &mut impl Rng,
    ) -> bool {
        free[j] = false;
        current.pairs.push((i, j));
        let stop = self.explore(free, current, cost.plus(self.pair_cost(i, j)), rng);
        current.pairs.pop();
        free[j] = true;
        stop
    }

    fn attempt_exemption(
        &mut self,
        i: usize,
        free: &mut [bool],
        current: &mut Solution,
        cost: Cost,
        rng: &mut impl Rng,
    ) -> bool {
        current.exempted_teams.push(i);
        let stop = self.explore(free, current, cost.plus(self.exemption_cost(i)), rng);
        current.exempted_teams.pop();
        stop
    }

    fn record(&mut self, current: &Solution, cost: Cost) -> bool {
        if cost < self.best_cost {
            self.best_cost = cost;
            self.best = current.clone();
        }
        self.best_cost <= self.floor
    }

    /// L'équipe libre au plus petit univers — *fail-first*. Si une impasse
    /// existe, on la rencontre tout de suite plutôt qu'après dix choix à
    /// défaire ; c'est ce qui fait tenir la recherche.
    fn most_constrained(&self, free: &[bool], rng: &mut impl Rng) -> Option<usize> {
        let mut candidates: Vec<usize> = Vec::new();
        let mut smallest = usize::MAX;
        for i in (0..self.n).filter(|&i| free[i]) {
            let size = self.universe_size(i, free);
            if size < smallest {
                smallest = size;
                candidates.clear();
            }
            if size == smallest {
                candidates.push(i);
            }
        }
        candidates.choose(rng).copied()
    }

    fn universe_size(&self, i: usize, free: &[bool]) -> usize {
        (0..self.n)
            .filter(|&j| j != i && free[j] && self.allowed[i][j])
            .count()
    }

    /// Les adversaires possibles de `i`, **du moins cher au plus cher**, puis
    /// l'exemption — qui coûte toujours davantage, R5 dominant.
    ///
    /// Le mélange précède un tri **stable** : l'ordre des coûts est respecté,
    /// et le sort ne départage que les égalités. C'est le tirage au sort, et c'est aussi ce
    /// qui fait que la première descente est déjà une bonne solution.
    fn universe(&self, i: usize, free: &[bool], rng: &mut impl Rng) -> Vec<Option<usize>> {
        let mut partners: Vec<usize> = (0..self.n)
            .filter(|&j| free[j] && self.allowed[i][j])
            .collect();
        partners.shuffle(rng);
        partners.sort_by_key(|&j| self.weight[i][j]);
        let mut options: Vec<Option<usize>> = partners.into_iter().map(Some).collect();
        options.push(None);
        options
    }

    fn exemption_cost(&self, i: usize) -> Cost {
        Cost {
            unpaired: 1,
            unfair_exemption: self.lag[i],
            ..Cost::ZERO
        }
    }

    fn pair_cost(&self, i: usize, j: usize) -> Cost {
        let (times, position) = self.weight[i][j];
        Cost {
            rematches: times,
            staleness: position,
            ..Cost::ZERO
        }
    }

    fn proposal(&self, input: &DrawInput) -> DrawProposal {
        DrawProposal {
            pairings: self.pairings(input),
            exempted: self.reported_exempted(input),
            optimum_proven: !self.budget_exhausted,
        }
    }

    /// **La plus servie parmi celles qui restent**, et non la première explorée.
    ///
    /// `exempted_teams` est un `Vec` : quand R4 force plusieurs équipes à rester sur
    /// le banc, chacune y entre. Rendre `.first()` désignait donc « l'exemptée »
    /// par un artefact de l'ordre de parcours, les autres disparaissant du
    /// rapport. Sous R8, celle qui se repose légitimement est celle qui a le plus
    /// joué — retard nul, ou le plus petit.
    ///
    /// Le coût, lui, **somme** les retards de toutes les restantes : c'est la
    /// bonne mesure d'équité quand plusieurs ne jouent pas.
    fn reported_exempted(&self, input: &DrawInput) -> Option<TeamId> {
        self.best
            .exempted_teams
            .iter()
            .min_by_key(|&&i| self.lag[i])
            .map(|&i| input.teams[i])
    }

    /// Reçoit l'équipe au plus petit identifiant — la première inscrite. C'est
    /// la règle de l'ancien tirage (`normalize_pair`), que la recherche ne doit
    /// pas changer : sans elle, l'ordre de parcours décidait qui reçoit.
    fn pairings(&self, input: &DrawInput) -> Vec<ProposedPairing> {
        self.best
            .pairs
            .iter()
            .map(|&(i, j)| {
                let (home, away) = pair(&input.teams[i], &input.teams[j]);
                ProposedPairing {
                    home,
                    away,
                    history: input.history.history(&home, &away),
                }
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
    fn teams(n: usize) -> Vec<TeamId> {
        let mut ids: Vec<TeamId> = (0..n).map(|_| TeamId::new()).collect();
        ids.sort_by_key(|id| id.to_string());
        ids
    }

    fn round(position: i32) -> (MatchDayPosition, MatchDayName) {
        (
            MatchDayPosition::try_new(position).unwrap(),
            MatchDayName::try_new(format!("Journée {position}")).unwrap(),
        )
    }

    fn input_of(teams: Vec<TeamId>) -> DrawInput {
        DrawInput {
            teams,
            ..DrawInput::default()
        }
    }

    fn record_played(input: &mut DrawInput, a: usize, b: usize, position: i32) {
        let (pos, name) = round(position);
        let (x, y) = (input.teams[a], input.teams[b]);
        input.history.record(&x, &y, pos, name);
    }

    fn set_matches_played(input: &mut DrawInput, team: usize, matches: u32) {
        input
            .matches_played
            .insert(input.teams[team], MatchesPlayed(matches));
    }

    fn seed(n: u64) -> StdRng {
        StdRng::seed_from_u64(n)
    }

    fn paired(p: &DrawProposal) -> HashSet<(TeamId, TeamId)> {
        p.pairings.iter().map(|r| pair(&r.home, &r.away)).collect()
    }

    // ── R5 — apparier le plus d'équipes possible ─────────────────────────────

    /// **Le test qui manquait.** Quatre équipes dont trois paires sont déjà
    /// jouées : `generate_round_pairings` n'en appariait qu'une, laissant deux
    /// équipes rentrer chez elles.
    ///
    /// Vérifié avant correction, sur la fonction en service :
    /// `obtenu : [("team-1", "team-4")]` — une rencontre au lieu de deux.
    #[test]
    fn the_smaller_id_always_plays_at_home() {
        let input = input_of(teams(6));
        for s in 0..30 {
            for r in draw(&input, &mut seed(s)).pairings {
                assert!(
                    r.home.to_string() < r.away.to_string(),
                    "graine {s} : {} reçoit {}",
                    r.away,
                    r.home
                );
            }
        }
    }

    #[test]
    fn four_teams_three_pairs_played_give_two_pairings() {
        let mut input = input_of(teams(4));
        record_played(&mut input, 0, 1, 1);
        record_played(&mut input, 0, 2, 2);
        record_played(&mut input, 1, 2, 3);

        let p = draw(&input, &mut seed(1));

        assert_eq!(p.pairings.len(), 2, "obtenu : {:?}", p.pairings);
        assert!(p.exempted.is_none());
        let rematches = p
            .pairings
            .iter()
            .filter(|r| r.history != History::FirstMeeting)
            .count();
        assert_eq!(rematches, 1, "une revanche est concédée, pas deux");
    }

    /// Une équipe non appariée coûte plus cher que n'importe quelle revanche :
    /// même toutes paires jouées, tout le monde joue.
    #[test]
    fn all_pairs_played_still_pairs_everyone() {
        let mut input = input_of(teams(4));
        for (a, b) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
            record_played(&mut input, a, b, 1);
        }

        let p = draw(&input, &mut seed(2));

        assert_eq!(p.pairings.len(), 2);
    }

    // ── R5 — rejouer le moins, et le plus anciennement ───────────────────────

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
    fn at_equal_count_the_lowest_rematch_total_wins() {
        let mut input = input_of(teams(4));
        for (a, b) in [(0, 1), (2, 3), (0, 2), (0, 3), (1, 2)] {
            record_played(&mut input, a, b, 1);
        }

        let p = draw(&input, &mut seed(3));

        let expected: HashSet<(TeamId, TeamId)> = HashSet::from([
            pair(&input.teams[0], &input.teams[2]),
            pair(&input.teams[1], &input.teams[3]),
        ]);
        assert_eq!(paired(&p), expected);
    }

    /// Entre deux revanches également coûteuses, la plus ancienne est préférée.
    /// 0-1 date de la journée 1, 0-2 de la journée 5 : c'est 0-1 qu'on reprend.
    #[test]
    fn between_two_rematches_the_oldest_is_preferred() {
        let mut input = input_of(teams(4));
        record_played(&mut input, 0, 1, 1);
        record_played(&mut input, 0, 2, 5);
        record_played(&mut input, 0, 3, 5);
        record_played(&mut input, 1, 2, 5);
        record_played(&mut input, 1, 3, 5);
        record_played(&mut input, 2, 3, 5);

        let p = draw(&input, &mut seed(4));

        assert!(
            paired(&p).contains(&pair(&input.teams[0], &input.teams[1])),
            "la rencontre la plus ancienne devait être reprise : {:?}",
            p.pairings
        );
    }

    /// Le motif de la revanche vient du domaine, pas de la vue.
    #[test]
    fn a_rematch_carries_its_count_and_last_round() {
        let mut input = input_of(teams(2));
        record_played(&mut input, 0, 1, 2);
        record_played(&mut input, 0, 1, 7);

        let p = draw(&input, &mut seed(5));

        assert_eq!(
            p.pairings[0].history,
            History::Rematch {
                times: MeetingCount(2),
                last_round: MatchDayName::try_new("Journée 7").unwrap(),
            }
        );
    }

    // ── Les effectifs dégénérés ──────────────────────────────────────────────

    /// Le raccourci de `tirer` doit rendre ce que la recherche rendrait : une
    /// équipe seule n'a personne à jouer, donc elle est exemptée.
    #[test]
    fn a_lone_team_is_exempted() {
        let input = input_of(teams(1));

        let p = draw(&input, &mut seed(0));

        assert!(p.pairings.is_empty());
        assert_eq!(p.exempted, Some(input.teams[0]));
    }

    #[test]
    fn an_empty_count_exempts_nobody() {
        let p = draw(&input_of(vec![]), &mut seed(0));

        assert!(p.pairings.is_empty());
        assert_eq!(p.exempted, None);
    }

    // ── R8 — l'exemption va à celle qui a le plus joué ───────────────────────

    #[test]
    fn the_exempted_team_is_the_one_that_played_most() {
        let mut input = input_of(teams(5));
        for (team, matches) in [(0, 3), (1, 4), (2, 3), (3, 7), (4, 2)] {
            set_matches_played(&mut input, team, matches);
        }

        for seed_n in 0..20 {
            let p = draw(&input, &mut seed(seed_n));
            assert_eq!(p.pairings.len(), 2);
            assert_eq!(p.exempted, Some(input.teams[3]));
        }
    }

    /// Une équipe absente de la table n'a rien joué : son retard est maximal,
    /// donc elle passe **en dernier** pour l'exemption. Le cas se produit dès la
    /// première journée, où la table est vide pour tout le monde.
    #[test]
    fn a_team_without_history_counts_zero_matches() {
        let mut input = input_of(teams(3));
        set_matches_played(&mut input, 0, 5);
        // Les équipes 1 et 2 ne figurent pas dans `matchs_joues`.

        for seed_n in 0..20 {
            let p = draw(&input, &mut seed(seed_n));
            assert_eq!(
                p.exempted,
                Some(input.teams[0]),
                "la seule qui a joué se repose, les deux autres jouent"
            );
        }
    }

    /// Départage — à égalité de matchs joués, plus rien ne départage, et le sort
    /// tranche. Sans quoi la même équipe serait exemptée à chaque journée d'une
    /// saison régulière, où tout le monde a le même compte.
    #[test]
    fn equal_matches_played_are_broken_by_lot() {
        let mut input = input_of(teams(5));
        for team in 0..5 {
            set_matches_played(&mut input, team, 4);
        }

        let exempted_teams: HashSet<Option<TeamId>> = (0..30)
            .map(|n| draw(&input, &mut seed(n)).exempted)
            .collect();

        assert!(
            exempted_teams.len() > 1,
            "trente graines n'ont produit qu'une seule exemptée : le sort ne joue pas"
        );
    }

    /// R8 cède devant R5 : elle est au rang 4, pas au rang 3. Trois équipes,
    /// la 0 a le plus joué — c'est donc elle qu'on voudrait exempter — mais
    /// 1-2 est déjà jouée, donc l'exempter coûterait une revanche. On exempte
    /// la 1 ou la 2, et R8 cède.
    #[test]
    fn r8_yields_to_the_rematch_count() {
        let mut input = input_of(teams(3));
        set_matches_played(&mut input, 0, 9);
        record_played(&mut input, 1, 2, 1);

        let p = draw(&input, &mut seed(6));

        assert_ne!(
            p.exempted,
            Some(input.teams[0]),
            "exempter la plus servie aurait coûté une revanche"
        );
        assert_eq!(
            p.pairings[0].history,
            History::FirstMeeting,
            "on préfère une rencontre inédite à l'exemption souhaitée"
        );
    }

    /// Quand R4 laisse plusieurs équipes sur le banc, l'exemptée rapportée est
    /// **la plus servie parmi elles**, et non la première explorée.
    ///
    /// Trois équipes d'un même coach et une quatrième : une seule rencontre est
    /// possible, deux équipes restent. Celle qui a le plus joué se repose ; la
    /// seconde est perdue, et le panneau de défection s'en occupe.
    #[test]
    fn the_reported_exempted_is_the_most_served_of_the_leftovers() {
        let mut input = input_of(teams(4));
        let (a, b, c) = (input.teams[0], input.teams[1], input.teams[2]);
        input.forbidden = HashSet::from([pair(&a, &b), pair(&a, &c), pair(&b, &c)]);
        set_matches_played(&mut input, 0, 1);
        set_matches_played(&mut input, 1, 8);
        set_matches_played(&mut input, 2, 2);

        for seed_n in 0..20 {
            let p = draw(&input, &mut seed(seed_n));
            assert_eq!(p.pairings.len(), 1, "une seule paire est autorisée");
            assert_eq!(
                p.exempted,
                Some(b),
                "la plus servie des restantes, pas la première explorée"
            );
        }
    }

    // ── R4 — deux équipes d'un même coach ne se rencontrent jamais ───────────

    /// Contrainte dure : elle tient **même au prix d'un match en moins**.
    #[test]
    fn a_forbidden_pair_is_never_paired_even_at_that_cost() {
        let mut input = input_of(teams(2));
        input.forbidden = HashSet::from([pair(&input.teams[0], &input.teams[1])]);

        let p = draw(&input, &mut seed(7));

        assert!(p.pairings.is_empty(), "obtenu : {:?}", p.pairings);
    }

    #[test]
    fn a_forbidden_pair_is_avoided_when_possible() {
        let mut input = input_of(teams(4));
        input.forbidden = HashSet::from([pair(&input.teams[0], &input.teams[1])]);

        for seed_n in 0..20 {
            let p = draw(&input, &mut seed(seed_n));
            assert_eq!(p.pairings.len(), 2);
            assert!(!paired(&p).contains(&pair(&input.teams[0], &input.teams[1])));
        }
    }

    // ── Départage — c'est un vrai tirage ─────────────────────────────────────

    /// Deux graines différentes donnent des appariements différents dès que
    /// plusieurs combinaisons sont également bonnes. Sans cette propriété, le
    /// bouton « Retirer au sort » ne ferait rien.
    #[test]
    fn two_seeds_give_different_pairings() {
        let input = input_of(teams(6));

        let seen_draws: HashSet<Vec<(TeamId, TeamId)>> = (0..30)
            .map(|g| {
                let p = draw(&input, &mut seed(g));
                let mut pairs: Vec<_> = paired(&p).into_iter().collect();
                pairs.sort_by_key(|(a, b)| (a.to_string(), b.to_string()));
                pairs
            })
            .collect();

        assert!(
            seen_draws.len() > 1,
            "trente tirages ont donné le même appariement"
        );
    }

    /// L'aléa départage, il ne dégrade pas : tous les tirages restent optimaux.
    #[test]
    fn randomness_never_degrades_the_result() {
        let mut input = input_of(teams(6));
        record_played(&mut input, 0, 1, 1);
        record_played(&mut input, 2, 3, 1);

        for seed_n in 0..30 {
            let p = draw(&input, &mut seed(seed_n));
            assert_eq!(p.pairings.len(), 3);
            let rematches = p
                .pairings
                .iter()
                .filter(|r| r.history != History::FirstMeeting)
                .count();
            assert_eq!(rematches, 0, "graine {seed_n} : {:?}", p.pairings);
        }
    }

    // ── Les bords ────────────────────────────────────────────────────────────

    #[test]
    fn fewer_than_two_teams_give_no_pairing() {
        assert!(draw(&input_of(vec![]), &mut seed(8)).pairings.is_empty());
        assert!(draw(&input_of(teams(1)), &mut seed(8)).pairings.is_empty());
    }

    #[test]
    fn an_odd_count_exempts_exactly_one_team() {
        let input = input_of(teams(7));
        let p = draw(&input, &mut seed(9));

        assert_eq!(p.pairings.len(), 3);
        assert!(p.exempted.is_some());
    }

    /// Il n'y a plus de plafond : la recherche tient en O(n²) de mémoire, et la
    /// base de développement porte déjà une saison à vingt-deux équipes.
    #[test]
    fn a_large_league_is_paired_without_ceiling() {
        for n in [22, 26, 31] {
            let p = draw(&input_of(teams(n)), &mut seed(10));
            assert_eq!(p.pairings.len(), n / 2, "{n} équipes");
            assert_eq!(p.exempted.is_some(), n % 2 == 1);
            assert!(p.optimum_proven, "{n} équipes : budget épuisé");
        }
    }

    // ── draw_pools — R10 ─────────────────────────────────────────────────────

    #[test]
    fn a_pool_with_a_single_present_gets_no_pairing() {
        let pools = [input_of(teams(1)), input_of(teams(4))];
        let proposals = draw_pools(&pools, &mut seed(3)).unwrap();
        assert!(proposals[0].pairings.is_empty());
        assert_eq!(proposals[0].exempted, Some(pools[0].teams[0]));
        assert_eq!(
            proposals[1].pairings.len(),
            2,
            "l'autre poule joue quand même"
        );
    }

    #[test]
    fn no_drawable_pool_is_refused() {
        let pools = [input_of(teams(1)), input_of(teams(0))];
        assert_eq!(
            draw_pools(&pools, &mut seed(3)),
            Err(DomainError::NotEnoughPresent)
        );
    }

    // ── Les six tests hérités de `generate_round_pairings` ───────────────────
    //
    // Ils portent sur des **propriétés** — nombre de rencontres, absence de
    // doublon, normalisation — et non sur des appariements nommés. C'est ce qui
    // rend leur reprise possible telle quelle : seuls les types changent, pas
    // une seule assertion. Ils n'ont jamais vu le cas qui casse, mais ils
    // décrivent ce qui ne doit pas régresser.

    #[test]
    fn four_teams_without_history_give_two_pairings() {
        let p = draw(&input_of(teams(4)), &mut seed(20));

        assert_eq!(p.pairings.len(), 2);
        let mut seen_teams: HashSet<TeamId> = HashSet::new();
        for r in &p.pairings {
            assert!(seen_teams.insert(r.home), "équipe appariée deux fois");
            assert!(seen_teams.insert(r.away), "équipe appariée deux fois");
        }
    }

    #[test]
    fn the_next_round_replays_no_pair_of_the_first() {
        let mut input = input_of(teams(4));
        let first_draw = draw(&input, &mut seed(21));
        assert_eq!(first_draw.pairings.len(), 2);
        for r in &first_draw.pairings {
            let (a, b) = (r.home, r.away);
            let (pos, name) = round(1);
            input.history.record(&a, &b, pos, name);
        }

        let second_draw = draw(&input, &mut seed(22));

        assert_eq!(second_draw.pairings.len(), 2);
        let prior = paired(&first_draw);
        for r in &second_draw.pairings {
            assert!(!prior.contains(&pair(&r.home, &r.away)), "paire répétée");
        }
    }

    #[test]
    fn three_rounds_exhaust_the_six_pairs_then_the_cycle_restarts() {
        let mut input = input_of(teams(4));
        let mut all_pairs: HashSet<(TeamId, TeamId)> = HashSet::new();

        for number in 1..=3 {
            let p = draw(&input, &mut seed(30 + number as u64));
            for r in &p.pairings {
                all_pairs.insert(pair(&r.home, &r.away));
                let (pos, name) = round(number);
                input.history.record(&r.home, &r.away, pos, name);
            }
        }

        assert_eq!(all_pairs.len(), 6, "les six paires devaient être épuisées");
        assert_eq!(draw(&input, &mut seed(34)).pairings.len(), 2);
    }

    #[test]
    fn three_teams_give_one_pairing() {
        assert_eq!(draw(&input_of(teams(3)), &mut seed(23)).pairings.len(), 1);
    }

    #[test]
    fn zero_or_one_team_gives_nothing() {
        assert!(draw(&input_of(vec![]), &mut seed(24)).pairings.is_empty());
        assert!(draw(&input_of(teams(1)), &mut seed(24)).pairings.is_empty());
    }

    #[test]
    fn normalisation_treats_ab_and_ba_as_one_pair() {
        let mut input = input_of(teams(4));
        let (pos, name) = round(1);
        let (a, b) = (input.teams[1], input.teams[0]);
        input.history.record(&a, &b, pos, name);

        let p = draw(&input, &mut seed(25));

        assert!(
            !paired(&p).contains(&pair(&input.teams[0], &input.teams[1])),
            "la paire enregistrée dans l'autre sens n'a pas été reconnue"
        );
    }
}
