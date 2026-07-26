# Phase 4 — Contrats de données (`competition-rules-form`)

## Particularité de cette page

`SaveRulesPayload` (`competitions/io/web/new_competition.rs:410`) désérialise
**directement** l'agrégat domaine depuis le JSON :

```rust
#[derive(Deserialize)]
pub struct SaveRulesPayload {
    pub season_name: String,
    #[serde(flatten)]
    pub rules: CompetitionRules,
}
```

Le contrat HTTP, la structure persistée en JSONB et le type domaine sont donc **un
seul et même type**. Un unique arbitrage vaut pour les trois — c'est l'objet de la
décision ci-dessous.

## Décision — forme du champ des départages

Le champ `additionnal_ranking_points: HashMap<String, u32>`
(`competitions/domain/competition_rules.rs:52`) est remplacé par une **liste ordonnée
dont l'ordre est la priorité**. Il est renommé `tiebreakers` : son nom actuel est un
contresens (il ne porte pas des points additionnels mais des critères de départage).
Aucune clause de compatibilité — le projet n'est pas en production, contrairement au
cas `diff_td` que la feature `ranking-bonus-points` avait dû préserver.

```json
"tiebreakers": [
  { "code": "diff_td",        "activated": true  },
  { "code": "nb_td",          "activated": false },
  { "code": "nb_td_conceded", "activated": true  },
  { "code": "nb_cas",         "activated": true  }
]
```

Une seule source de vérité pour la priorité : la position dans la liste. Ni priorité
en doublon, ni trou dans la numérotation, ni divergence possible entre l'ordre et un
rang stocké. Le rang **affiché** se recalcule à partir de la position des seuls
critères actifs (cf. `02-front.md`).

Formes écartées : priorité explicite en plus de la position (deux sources de vérité
pour la même information, invariants de contiguïté à valider) ; map `code → { priorité,
activation }` (un objet JSON n'a pas d'ordre garanti, tri obligatoire à chaque lecture).

## Value objects — domaine `competitions`

```rust
/// Code d'un critère de départage. Validé en forme seulement : le domaine ne
/// connaît pas le catalogue (cf. 03-back.md, option a). L'appartenance au
/// catalogue est vérifiée par le use case via ITiebreakCatalogPort.
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, AsRef)
)]
pub struct TiebreakCode(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TiebreakSetting {
    pub code:      TiebreakCode,
    pub activated: Activated,     // VO existant — competition_rules.rs:8
}

/// Liste ordonnée : l'index porte la priorité. Smart constructor obligatoire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TiebreakConfig(Vec<TiebreakSetting>);
```

`Activated` est réutilisé tel quel — c'est déjà le VO d'activation des trois bonus,
une seule sémantique d'activation dans l'agrégat.

`TiebreakConfig::try_new(Vec<TiebreakSetting>) -> Result<Self, DomainError>` porte les
invariants du domaine :

| Invariant | Erreur | Règle |
|---|---|---|
| Liste non vide | `EmptyTiebreakConfig` | — |
| Au moins un `activated` | `NoActiveTiebreaker` | Règle 1 |
| Pas de doublon de code | `DuplicateTiebreakCode { code }` | — |

Le détail de l'implémentation (signature exacte, accesseurs, tests) est spécifié en
phase 6.

## DTO d'entrée (POST)

`SaveRulesPayload` est inchangé dans sa forme : c'est `CompetitionRules.ranking_rules`
qui change, en remplaçant `additionnal_ranking_points` par `tiebreakers: TiebreakConfig`.

La désérialisation d'une `TiebreakConfig` invalide (liste vide, doublon, aucun actif)
doit **échouer** — le smart constructor est traversé à la désérialisation, pas
court-circuité. Un `Json<SaveRulesPayload>` malformé produit donc un 422 avant même
d'atteindre le use case.

## DTO de sortie (GET)

`NewCompetitionPhase2Template` (`new_competition.rs:76`) gagne un champ :

```rust
pub struct NewCompetitionPhase2Template {
    pub app_routes:          AppRoutes,
    pub space_id:            String,
    pub competition_id:      String,
    pub season_id:           String,
    pub season_name_value:   String,
    pub existing_rules_json: String,
    pub tiebreak_catalog_json: String,   // ← nouveau
}
```

Pas de VM structuré : la section est rendue côté client (cf. `02-front.md`, D1), le
template n'a besoin que du catalogue sérialisé pour amorcer l'état JS. Le handler
projette les `TiebreakCriterionDto` du port en JSON — le DTO de port n'atteint jamais
le template, conformément à la règle « Domain services pour données inter-BCs ».

`existing_rules_json` est inchangé dans son principe : il sérialise `CompetitionRules`,
et porte donc désormais `tiebreakers` sous sa nouvelle forme. Le JS y lit ordre **et**
activation en une passe.

## DTO de port

```rust
// app/competitions/ports.rs
pub struct TiebreakCriterionDto {
    pub code:  String,     // primitives acceptées : DTO de lecture
    pub label: String,
}

pub trait ITiebreakCatalogPort: Send + Sync {
    fn all(&self) -> Vec<TiebreakCriterionDto>;
}
```

## Interfaces d'utilisation

| DTO / type | Émis par | Consommé par |
|---|---|---|
| `TiebreakCriterionDto` | `tiebreak_catalog_adapter` (infrastructure) | Handler GET (projection JSON) + use case `save_competition_rules` (validation d'appartenance) |
| `tiebreak_catalog_json` | Handler GET `get_new_competition_phase_2` | Template → JS, amorce de l'état `criteriaOrder` |
| `existing_rules_json` | Handler GET (existant) | Template → JS, `initFromExistingRules` (ordre + activation) |
| `TiebreakConfig` | JS `buildJSON()` → `SaveRulesPayload` | Use case → agrégat `RankingRules` → JSONB |
| `TiebreakConfig` (lecture) | Repository (JSONB) | Handler GET via `existing_rules_json` ; unité `tiebreak-calc` via l'ACL `competition_info_adapter` |

## Erreurs applicatives

`SaveCompetitionRulesError` (`use_cases/save_competition_rules.rs:15`) gagne :

| Variante | Origine | Réponse HTTP |
|---|---|---|
| `NoActiveTiebreaker` | Domaine, via `TiebreakConfig::try_new` | 422 + message |
| `DuplicateTiebreakCode { code }` | Domaine | 422 + message |
| `UnknownTiebreakCriterion { code }` | Use case, après consultation du port | 422 + message |

Les messages suivent le style existant du handler (`RosterInMultipleTiers` produit une
phrase française explicite, pas un code).

## Règles métier — état

Aucune règle nouvelle à cette étape. La règle 1 se matérialise en invariant du VO
`TiebreakConfig`. Pour la règle 3 (7 critères actifs par défaut), cf. la correction
apportée en `05-use-cases.md` : elle vit dans l'amorce front, aucune configuration
n'étant persistée avant la première soumission ; le domaine fournit néanmoins un
`TiebreakConfig::default()` pour les lecteurs d'une configuration absente.
