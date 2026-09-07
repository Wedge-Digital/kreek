# Le tirage rejoue plutôt que de laisser une équipe sur le banc

**Priorité : haute — le défaut touche le Calendrier en production**
**Épic :** E16 — Sondage de présence
**Dépend de :** rien
**Fichiers :** `src/app/competitions/domain/tirage.rs` (nouveau),
`src/app/competitions/domain/match_day.rs`
**Spec :** `docs/specs/sondage-presence/onglet-presences/06-domaine.md`

## Le défaut

`generate_round_pairings(teams, already_played)` sert aujourd'hui l'onglet
Calendrier. Il a deux défauts, et ni l'un ni l'autre n'est visible dans ses six
tests.

**Il n'est pas aléatoire.** Aucun `shuffle`, aucun générateur : il énumère les
paires dans l'ordre des indices et les prend gloutonnement. Cinq appels sur la
même entrée rendent cinq fois le même appariement.

**Il préfère ne pas apparier plutôt que de programmer une revanche.** Le repli
sur l'ensemble complet des paires n'intervient que si la liste des inédites est
*vide* ; tant qu'elle en contient une, le glouton la prend et peut condamner les
équipes restantes :

```
4 équipes, déjà joué A-B, A-C, B-C  ->  un seul match, A contre D
                                        B et C repartent sans jouer
```

Mesuré sur des historiques où la moitié des paires est jouée — mi-saison :

| Présents | Tirages laissant des équipes non appariées |
|---|---|
| 6 | 53,9 % |
| 8 | 57,8 % |
| 10 | 56,4 % |

**Et `already_played` est un `HashSet`**, donc binaire : on sait « déjà jouée »,
jamais « jouée trois fois ». Minimiser suppose de compter — c'est ce choix de
type, pas une omission de logique, qui rendait la minimisation impossible.

## L'ordre des objectifs — R8, R9, R10, R17

| Rang | Règle | Nature |
|---|---|---|
| 1 | R10 — jamais deux équipes d'un même coach | contrainte dure, jamais violable |
| 2 | R8.1 — apparier le plus d'équipes possible | objectif |
| 3 | R8.2 — rejouer le moins, et le plus anciennement | objectif |
| 4 | R9 — exempter une équipe qui ne l'a jamais été | préférence, si effectif impair |
| 5 | R17 — départager au sort | arbitrage final |

Seul le rang 1 refuse ; les autres cèdent dans l'ordre. **Une revanche est
préférable à une équipe qui rentre chez elle sans avoir joué** — c'est l'inverse
de ce que le code fait aujourd'hui.

## Conception

`domain/tirage.rs`, fonction pure :

```rust
pub fn tirer(input: &DrawInput, rng: &mut impl Rng) -> DrawProposal

pub struct DrawInput {
    pub equipes:          Vec<TeamId>,
    pub historique:       RencontresJouees,          // comptes par paire — R8
    pub interdites:       HashSet<(TeamId, TeamId)>, // R10
    pub jamais_exemptees: HashSet<TeamId>,           // R9
}
```

**Le générateur est un paramètre.** C'est ce qui rend R8 testable : un `StdRng`
à graine fixe donne un tirage reproductible en test, `from_os_rng()` en
production. `random_draw.rs` isole la partie déterministe du `shuffle` non
testé ; l'injection fait mieux, en rendant testable l'ensemble.

**Énumération avec élagage**, pas de couplage maximum pondéré. À vingt équipes
au plus, elle suffit ; écrire l'algorithme optimal serait payer une complexité
pour un cas qui n'existe pas dans une ligue amateur.

`generate_round_pairings` **reste en place** dans cette carte — c'est la 508 qui
la retire, après avoir migré son appelant. Livrer les deux d'un coup mêlerait un
algorithme neuf et une migration dans le même diff.

## Checklist

- [ ] **Écrire d'abord le test qui manque, et le voir échouer** : quatre équipes,
      déjà joué A-B A-C B-C, attendre **deux** rencontres dont une revanche.
      C'est le cas qu'aucun des six tests actuels ne couvre.
- [ ] `RencontresJouees` — comptes par paire, avec la journée de la dernière
- [ ] `tirer` : R10 en filtre préalable, puis maximisation, puis minimisation
- [ ] R9 — l'exemptée tirée parmi celles qui ne l'ont jamais été
- [ ] R17 — départage au sort des combinaisons ex æquo
- [ ] `Historique::{Inedite, Revanche { fois, derniere }}` sur chaque rencontre :
      le tirage sait *pourquoi* il a concédé, il le dit
- [ ] Tests : R8.1, R8.2 (moindre total), R8.2 (la plus ancienne), R9, R10 même
      au prix d'un match en moins, R17 sur deux graines
- [ ] `make lint`, `make check-arch`, `make test`
