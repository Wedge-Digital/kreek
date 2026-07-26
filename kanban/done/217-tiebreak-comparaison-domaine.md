# Départages — Comparaison et rangs dans le domaine (additif)

**Priorité : haute**
**Dépend de :** carte 216 (compteurs dans `CumulativeTotals`)
**Contexte :** `src/app/ranking/domain/standings.rs` (nouveau), `src/app/ranking/domain/tiebreak.rs`
**Spec :** `docs/specs/ranking/tiebreakers/tiebreak-calc/06-domaine.md`

## Objectif

Écrire la logique de comparaison et d'attribution des rangs dans le domaine. **Purement
additif** : personne n'appelle ces fonctions avant la carte 218, le classement affiché ne
change pas. Commit intermédiaire sûr.

## Conception (cf. `06-domaine.md`)

### `tiebreak.rs` — trois fonctions

```rust
pub enum Direction { Asc, Desc }

impl TiebreakCriterion {
    pub fn from_code(code: &str) -> Option<Self>;              // symétrique de code()
    pub fn direction(&self) -> Direction;                       // règle 17
    pub fn value_of(&self, totals: &CumulativeTotals) -> i64;   // signé !
}
```

| Critère | `value_of` | `direction` |
|---|---|---|
| `DiffTd` | `td_for − td_against` | Desc |
| `NbTd` | `td_for` | Desc |
| `NbTdConceded` | `td_against` | **Asc** |
| `NbCas` | `casualties` | Desc |
| `NbWins` | `wins` | Desc |
| `NbFouls` | `fouls` | Desc |
| `NbReu` | `completions` | Desc |

**`i64` et non `u32`** : `diff_td` peut être négatif. En non signé, une équipe à −3 de
différence obtiendrait un compteur de 4 milliards et passerait en tête.

### `standings.rs` — nouveau

```rust
pub struct TeamStanding { pub team_id: TeamId, pub totals: CumulativeTotals }
pub struct TiebreakOrder(Vec<TiebreakCriterion>);

pub fn compare(a: &TeamStanding, b: &TeamStanding, order: &TiebreakOrder) -> Ordering;
pub fn order_standings(standings: &mut [TeamStanding], order: &TiebreakOrder);
pub fn assign_ranks(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<Rank>;
```

- `compare` : `ranking_points` décroissants d'abord, puis chaque critère actif dans
  l'ordre jusqu'au premier qui départage (règle 18) ; `Equal` si tous sont égaux (règle 19).
- `order_standings` : `sort_by` — **tri stable**. Deux équipes strictement ex æquo doivent
  conserver leur ordre d'entrée, sinon le classement permute d'un rafraîchissement à
  l'autre sans qu'aucun match ait été joué.
- `assign_ranks` : « même rang que le précédent si `compare` renvoie `Equal`, sinon
  `idx + 1` » — produit la numérotation standard 1, 2, 2, 4 (règle 20) sans cas
  particulier ni compteur d'ex æquo.

Un `TiebreakOrder` vide est un état valide : l'ordre se réduit aux points.

**Aucune erreur domaine** : rien ici ne peut échouer, il n'y a que des valeurs à comparer.

## Tests (15 cas, cf. `06-domaine.md`)

`tiebreak.rs` : `from_code` résout les 7 codes et renvoie `None` sur l'inconnu ; il est
l'inverse exact de `code()` ; `NbTdConceded` est le seul `Asc` ; `value_of` lit le bon
compteur pour les 7 ; `value_of(DiffTd)` est **négatif** quand l'équipe encaisse plus
qu'elle ne marque.

`standings.rs` : les points priment sur tous les critères ; le 1ᵉʳ critère départage ;
1ᵉʳ égal ⇒ le 2ᵉ départage ; tous égaux ⇒ `Equal` ; ordre vide ⇒ seuls les points comptent ;
`NbTdConceded` favorise qui encaisse le moins ; `order_standings` est stable ;
`assign_ranks` donne 1, 2, 2, 4 ; toutes ex æquo ⇒ toutes au rang 1 ; une seule équipe ⇒ 1.

## Checklist

- [ ] `from_code`, `direction`, `value_of` sur `TiebreakCriterion`
- [ ] `value_of` retourne un `i64`, vérifié par un test sur une différence négative
- [ ] `standings.rs` avec `TeamStanding`, `TiebreakOrder`, `compare`, `order_standings`, `assign_ranks`
- [ ] Tri stable, vérifié par un test
- [ ] Les 15 tests écrits et verts
- [ ] Aucun appelant en production (le câblage est en 218)
- [ ] `make test` + `make check-arch` passent
