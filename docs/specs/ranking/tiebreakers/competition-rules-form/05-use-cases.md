# Phase 5 — Use cases (`competition-rules-form`)

## Une seule mutation

La page n'a qu'une mutation : l'enregistrement des règles. Le use case
`save_competition_rules::execute` existe déjà — il est **étendu**, aucun use case
nouveau n'est créé (la saisie des départages n'est pas une opération métier distincte
de la saisie des règles).

## Signature

```rust
pub async fn execute(
    cmd:     SaveCompetitionRulesCommand,
    repo:    &dyn ISeasonRepository,
    catalog: &dyn ITiebreakCatalogPort,   // ← nouveau
) -> Result<(), SaveCompetitionRulesError>
```

`SaveCompetitionRulesCommand` est **inchangé** : la configuration des départages voyage
dans `cmd.rules.ranking_rules.tiebreakers`, déjà validée en forme par le smart
constructor traversé à la désérialisation (cf. `04-dtos.md`).

## Orchestration

```
1. Vérifier l'unicité des rosters entre tiers        (existant)
2. Vérifier que chaque code de départage est au catalogue   ← nouveau, via le port
3. Persister les règles
```

L'ordre importe peu — les deux vérifications sont indépendantes — mais le pas 2 est
placé après le pas 1 pour ne pas modifier le comportement observable des cas déjà
testés.

`execute` fait aujourd'hui exactement 20 lignes (`save_competition_rules.rs:33-52`).
Ajouter la vérification en ligne franchirait la limite : la boucle d'unicité des
rosters est **extraite** dans une fonction nommée, et la nouvelle vérification en est
une seconde.

```rust
fn ensure_roster_unicity(rules: &CompetitionRules) -> Result<(), SaveCompetitionRulesError>
fn ensure_known_tiebreak_codes(
    config:  &TiebreakConfig,
    catalog: &dyn ITiebreakCatalogPort,
) -> Result<(), SaveCompetitionRulesError>
```

`execute` se réduit alors à trois appels, conformément à la règle des 20 lignes.

`ensure_known_tiebreak_codes` compare les codes soumis à `catalog.all()` et lève
`UnknownTiebreakCriterion { code }` au premier inconnu. Elle ne vérifie **pas**
l'exhaustivité : une configuration qui omettrait un critère du catalogue est valide
(elle sera complétée à l'hydratation, cf. `02-front.md`).

## Répartition des validations — rappel

| Vérification | Où | Quand |
|---|---|---|
| Liste non vide, au moins un actif, pas de doublon | Domaine (`TiebreakConfig::try_new`) | À la désérialisation du payload — 422 avant le use case |
| Code appartenant au catalogue | **Use case**, via `ITiebreakCatalogPort` | Pas 2 de l'orchestration |
| Unicité des rosters entre tiers | Use case (existant) | Pas 1 |

Le use case ne décide rien de métier : il consulte une donnée externe (le catalogue)
et délègue le reste au domaine.

## Erreurs

`SaveCompetitionRulesError` gagne les trois variantes listées en `04-dtos.md`. Deux
proviennent du domaine et ne peuvent survenir qu'en désérialisation ; seule
`UnknownTiebreakCriterion { code }` est levée par le use case lui-même.

## Sites d'appel à adapter

| Site | Nature | Adaptation |
|---|---|---|
| `post_competition_rules` (`new_competition.rs:418`) | prod | Passe `state.competitions.tiebreak_catalog_port.as_ref()` en 3ᵉ argument, et mappe la nouvelle erreur en 422 |
| `execute_save_rules` — tests du use case | test | Fournir un faux catalogue (les 7 codes) |
| `base_rules()` (`save_competition_rules.rs:150`, helper de test) | test | `additionnal_ranking_points: HashMap::new()` → `tiebreakers: TiebreakConfig` par défaut |
| `rules()` (`rules_labels.rs:44`, helper de test) | test | Idem |
| `legacy_rules_without_new_fields_deserialize_with_defaults` (`competition_rules.rs:121`) | test | Le JSON de fixture porte `additionnal_ranking_points` — à renommer en `tiebreakers` avec la nouvelle forme |

## Correction d'un constat des phases précédentes

Le `README.md` de la feature affirmait que « le défaut à la création est une map vide
(`save_competition_rules.rs:171`, `rules_labels.rs:64`) ». C'est **inexact** : ces deux
sites sont des **helpers de test** (`mod tests`), pas du code de production.

En réalité **aucun défaut n'existe** : `create_draft_competition` n'écrit pas de règles,
et `find_rules` renvoie `None` jusqu'à la première soumission de la phase 2 — le
handler GET produit alors `existing_rules_json = "null"` et c'est le front qui amorce
la liste depuis le catalogue.

Conséquence pour la **règle 3** (les 7 critères actifs par défaut) : elle se matérialise
dans l'amorce front, pas dans une valeur par défaut persistée. Le domaine fournit
néanmoins un `TiebreakConfig::default()` (les 7 codes, tous actifs, ordre canonique)
pour les consommateurs qui liraient une configuration absente — au premier chef l'ACL
`competition_info_adapter.rs:34`, qui alimentera l'unité `tiebreak-calc`.

## Règles métier — état

Aucune règle nouvelle. La correction ci-dessus précise **où** vit la règle 3, elle ne
la change pas.
