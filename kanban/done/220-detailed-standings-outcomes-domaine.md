# Classement détaillé — Résolution du critère décisif dans le domaine

**Priorité : haute**
**Dépend de :** —
**Contexte :** `src/app/ranking/domain/standings.rs`
**Spec :** `docs/specs/ranking/tiebreakers/detailed-standings/06-domaine.md`

## Objectif

Déterminer, pour chaque équipe, **quel critère l'a départagée** des équipes partageant son
total de points. **Purement additif** : aucun appelant avant la carte 223, aucun affichage
ne change. Implémente les règles 21 et 22.

## Conception

```rust
/// Ce qui a décidé de la position d'une équipe parmi celles qui partagent son
/// total de points de classement.
pub enum RowTiebreak {
    /// Seule à son total : aucune égalité à résoudre.
    Alone,
    /// Départagée par le critère d'index donné ; tous ceux qui le précèdent
    /// étaient égaux au sein de son sous-groupe (règle 21).
    DecidedBy(usize),
    /// Tous les critères actifs sont égaux — ex æquo (règles 19 et 22).
    FullyTied,
}

/// Un élément par équipe, dans l'ordre du tableau reçu. Attend un tableau **déjà
/// ordonné** par `order_standings`.
pub fn tiebreak_outcomes(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<RowTiebreak>;
```

### Algorithme — descente par sous-groupes

1. Découper en suites d'équipes à `ranking_points` égal. Une suite d'une seule équipe
   donne `Alone`.
2. Pour une suite de 2+, au critère d'index `k` :
   - `k` au-delà du dernier critère actif ⇒ toute la suite est `FullyTied` ;
   - sinon découper la suite en sous-suites de même valeur pour le critère `k` ;
   - sous-suite d'une seule équipe ⇒ `DecidedBy(k)` : ce critère l'a isolée ;
   - sous-suite de 2+ ⇒ retour à l'étape 2 avec `k + 1` — ce critère ne les a pas
     départagées.

**Pas de résolution à plat.** Marquer « décisif » le premier critère non constant du
groupe entier désignerait ce critère sur des lignes qu'il n'a pas départagées : sur
`+5 / +2 / +2`, les deux équipes à `+2` recevraient une mise en évidence sur deux valeurs
identiques.

Aucune évolution de `compare`, `order_standings` ni `assign_ranks` : `tiebreak_outcomes`
lit `TiebreakCriterion::value_of` sur des données déjà produites.

Aucun value object nouveau, aucune variante de `DomainError` — rien ne peut échouer.

## Tests

| Test | Règle |
|---|---|
| Équipe seule à son total ⇒ `Alone` | — |
| Deux équipes séparées par le 1ᵉʳ critère ⇒ `DecidedBy(0)` des deux côtés | 21 |
| 1ᵉʳ critère égal, le 2ᵉ sépare ⇒ `DecidedBy(1)` | 21 |
| Tous les critères actifs égaux ⇒ `FullyTied` | 22 |
| **Sous-groupe** : `+5 / +2 / +2` ⇒ la 1ʳᵉ `DecidedBy(0)`, les deux autres résolues au critère suivant | 21 |
| Ordre de départage vide + égalité de points ⇒ `FullyTied` | 19 |
| Deux totaux de points distincts ⇒ les groupes ne se mélangent pas | 18 |

**Le test de sous-groupe s'écrit en premier et doit être vu échouer** sur une
implémentation à plat. C'est le seul qui distingue la version révisée de R21 de la
formulation initiale — les deux coïncident sur un groupe de deux équipes.

## Checklist

- [ ] `RowTiebreak` et `tiebreak_outcomes` dans `standings.rs`
- [ ] Résolution par sous-groupes, pas à plat
- [ ] Les 7 tests, celui de sous-groupe écrit en premier et vu échouer sur la variante naïve
- [ ] Aucun appelant en production (le câblage est en 223)
- [ ] `make test` + `make check-arch` passent
