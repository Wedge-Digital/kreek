# Départages — Catalogue des critères dans le BC `ranking`

**Priorité : haute**
**Dépend de :** —
**Contexte :** `src/app/ranking/domain/`, `src/app/ranking/io/web/`
**Spec :** `docs/specs/ranking/tiebreakers/competition-rules-form/03-back.md`

## Objectif

Créer le catalogue des 7 critères de départage dans le BC `ranking`, qui en est le
propriétaire. **Purement additif** : aucun consommateur à ce stade, aucun comportement
modifié. Commit intermédiaire sûr.

## Conception (cf. `03-back.md`)

### `app/ranking/domain/tiebreak.rs` — nouveau

```rust
pub enum TiebreakCriterion {
    DiffTd, NbTd, NbTdConceded, NbCas, NbWins, NbFouls, NbReu,
}

impl TiebreakCriterion {
    /// Ordre canonique — sert de défaut d'affichage et de priorité initiale.
    pub fn all() -> Vec<Self>
    /// Identifiant stable, utilisé en JSON et en persistance côté competitions.
    pub fn code(&self) -> &'static str
}
```

Codes : `diff_td`, `nb_td`, `nb_td_conceded`, `nb_cas`, `nb_wins`, `nb_fouls`, `nb_reu`.
Ils sont **stables** : la configuration persistée dans `competition_seasons.rules` les
référence.

`nb_red_cards` est volontairement **absent** — `MatchActionType` n'expose ni carton
rouge ni expulsion, le critère vaudrait 0 pour toutes les équipes (règle 10). À
réintroduire le jour où les expulsions seront saisies dans le rapport de match.

### `app/ranking/io/web/tiebreak_labels.rs` — nouveau

Libellés français, sur le modèle de `competitions/io/web/rules_labels.rs` (le libellé
est de la présentation, il ne va pas dans le domaine) :

| Code | Libellé |
|---|---|
| `diff_td` | Différence de touchdowns (marqués − encaissés) |
| `nb_td` | Nombre de touchdowns marqués |
| `nb_td_conceded` | Nombre de touchdowns encaissés |
| `nb_cas` | Nombre de blessures infligées |
| `nb_wins` | Nombre de victoires |
| `nb_fouls` | Nombre de fautes commises |
| `nb_reu` | Nombre de réussites |

### Hors périmètre

Le **sens de comparaison** et les **compteurs cumulés** (règles 11 à 19) appartiennent à
l'unité `tiebreak-calc`. Cette carte ne livre que l'énumération, l'ordre canonique, les
codes et les libellés.

## Checklist

- [ ] `TiebreakCriterion` avec 7 variantes, `all()` dans l'ordre canonique, `code()` stable
- [ ] `nb_red_cards` absent du catalogue
- [ ] Libellés dans `io/web/tiebreak_labels.rs`, pas dans le domaine
- [ ] Tests unitaires : `all()` renvoie 7 entrées dans l'ordre, codes tous distincts, un libellé par code
- [ ] `make test` + `make check-arch` passent
