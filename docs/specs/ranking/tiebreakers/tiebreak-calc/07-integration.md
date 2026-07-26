# Phase 7 — Effets de bord (`tiebreak-calc`)

## Persistance

### Migration — 5 colonnes

`migrations/<ts>_ranking_lines_tiebreak_counters.sql` :

```sql
ALTER TABLE ranking_lines
    ADD COLUMN td_for      INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN td_against  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN casualties  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN fouls       INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN completions INTEGER NOT NULL DEFAULT 0;
```

Pas de colonne `diff_td` (dérivé), pas de backfill : hors production, et la projection est
rebuildable depuis l'event store.

**Conséquence en développement** : les lignes déjà écrites auront tous leurs compteurs à
0. Deux équipes à égalité de points y seront donc ex æquo sur tous les critères, quels que
soient les matchs réellement joués. Un `make reset_db` avant de tester le départage évite
de conclure à un bug. À signaler à l'utilisateur, pas à exécuter d'office.

### Repository

| Élément | Changement |
|---|---|
| `find_latest_line`, `find_latest_lines_for_season` | 5 colonnes ajoutées au `SELECT` |
| `insert_lines` | 5 colonnes ajoutées à l'`INSERT` (`$14` à `$18`) |
| Méthodes du port | **aucune nouvelle** |
| Cache `.sqlx/` | à régénérer (`make prepare_db`) après application de la migration |

`sample_line` (helper de test du repository) gagne les 5 valeurs — il portait déjà
`#[allow(clippy::too_many_arguments)]`, l'ajout ne dégrade pas la situation mais un
regroupement en struct serait le bon réflexe si la liste s'allonge encore.

## Événements

**Aucun événement nouveau, aucun listener nouveau.** Le payload
`MatchReportPublished` porte déjà toutes les actions.

`match_report_published_listener` gagne deux fonctions de comptage, calquées sur
`count_sorties` (`:79`) :

```rust
fn count_agressions(actions: &[MatchActionPublishedPayload]) -> FoulsCommitted
fn count_passes(actions: &[MatchActionPublishedPayload]) -> CompletionsMade
```

Filtrage en couche IO : le domaine ne connaît pas les types du payload, il reçoit des
nombres. Le listener construit les deux `TeamMatchStats` de la commande.

## Handlers

### `classement_widget::build_vm` (`:65`)

Fait **17 lignes** aujourd'hui. Il charge déjà les règles ; il doit en plus construire le
`TiebreakOrder` et appeler le domain service. Pour rester sous la limite des 20 lignes, la
construction de l'ordre et l'appel au service sont extraits :

```rust
fn tiebreak_order_of(rules: &Option<RankingRulesInfo>) -> TiebreakOrder
```

Aucune route nouvelle, aucun widget nouveau, aucun endpoint modifié.

### `builders.rs`

| Fonction | Changement |
|---|---|
| `build_classement_rows` (`:79`, **33 lignes**) | Reçoit des `(TeamStanding, Rank)` déjà ordonnés ; **perd** son `sort_by` (`:106`) et sa boucle de rangs. Elle passe donc sous la limite des 20 lignes — dette préexistante résorbée au passage |
| `build_group_vm`, `build_classement_groups` | Signature adaptée au nouveau type d'entrée ; découpage par groupe inchangé |

L'ordonnancement s'applique **par groupe**, chaque groupe étant un classement autonome.

## Templates

`classement-widget.html` : **aucun changement structurel**. Deux conséquences visuelles à
assumer, dues à l'ex æquo :

1. Des numéros de rang **identiques apparaissent sur des lignes consécutives** (1, 2, 2,
   4). C'est le comportement voulu (règle 20).
2. La ligne 39 affiche un trophée conditionnellement : `{% if row.rank == 1 %}🏆{% endif %}`.
   Deux équipes ex æquo au rang 1 recevront donc **chacune** le trophée. C'est correct —
   elles sont premières toutes les deux — mais c'est un changement d'affichage qu'il faut
   avoir vu venir.

## Tests

### Unitaires

Les 24 tests listés en `06-domaine.md`.

### Intégration

`test_match_report_published_pipeline.rs` (`#[sqlx::test]`) est étendu : le payload gagne
des actions `Agression` et `Passe`, et le test vérifie que les 5 compteurs sont persistés
avec les bonnes valeurs pour chaque équipe — c'est le test qui exerce le chemin complet
listener → use case → domaine → base, donc celui qui attraperait une inversion de
croisement échappée aux tests unitaires.

### E2E

Nouveau fichier `tests/e2e/test_ranking_tiebreak.py`, calqué sur `test_ranking_bonus.py`
qui fournit déjà la création d'une compétition à règles personnalisées et la publication
de rapports de match par API.

| Scénario | Vérifie |
|---|---|
| Départage par le 1ᵉʳ critère | Compétition avec `diff_td` seul actif ; deux équipes terminent à égalité de points avec des différences de TD distinctes → l'ordre du widget Classement suit la différence de TD |
| Ex æquo total | Deux équipes égales sur les points **et** sur tous les critères actifs → le widget affiche **deux fois le même rang** |
| Numérotation standard | Après deux ex æquo au rang 2, l'équipe suivante affiche le rang **4** (règle 20) |

## Règles métier — état

Aucune règle nouvelle. La phase constate deux effets d'affichage de la règle 20 (rangs
répétés, trophée possiblement multiple) qui n'étaient pas explicités jusqu'ici.
