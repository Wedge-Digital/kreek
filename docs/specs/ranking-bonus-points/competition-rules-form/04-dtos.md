# Phase 4 — Contrats de données (competition-rules-form)

## DTO d'entrée (command)

**Aucune nouvelle struct DTO** : le POST désérialise directement dans le domaine via
`SaveRulesPayload { #[serde(flatten)] rules: CompetitionRules, .. }`. Les nouveaux
champs sont des **value objects** (côté command ⇒ pas de primitive nue, validés à la
désérialisation).

```rust
// DefensiveBonus — champ ajouté
pub max_td_conceded: MaxTdConceded,     // VO, deser depuis u32

// AggressiveBonus — nouvelle struct
pub struct AggressiveBonus {
    pub activated: Activated,           // VO existant
    pub points: RankingPoints,          // VO existant réutilisé
    pub min_casualties: MinCasualties,  // VO nouveau, deser depuis u32
}

// RankingRules — champ ajouté
pub aggressive_bonus: AggressiveBonus,
```

Bornes de validation des VO (`MaxTdConceded`, `MinCasualties`) : fixées en Phase 6.

### Rétro-compat désérialisation JSONB

Règles enregistrées avant la feature (colonne `rules JSONB`) ne contiennent pas les
nouveaux champs. Défauts appliqués via `#[serde(default = "…")]` :

| Champ absent | Défaut | Justification |
|---|---|---|
| `defensive_bonus.max_td_conceded` | `1` | comportement historique « ≤ 1 TD encaissé » |
| `ranking_rules.aggressive_bonus` | `{ activated: false, points: 1, min_casualties: 2 }` | bonus nouveau, désactivé par défaut ⇒ 0 point |

## DTO de sortie (query / présentation)

**Aucune nouvelle struct VM.** Le récap consomme un `Option<String>` produit par le
helper `format_bonus_label(&RankingRules)`. Les templates phase-5 et summary_tab
possèdent déjà un champ label bonus (`Option<String>`) — seule la valeur change
(seuil défensif configurable + éventuelle mention agressif).

Les VMs de lecture peuvent utiliser des primitives (`Option<String>`) — conforme
CLAUDE.md (side query).

## DTOs de port

Aucun pour cette unité (intra-`competitions`).

## Interfaces d'utilisation (émetteur → consommateur)

| DTO | Émetteur | Consommateur(s) |
|---|---|---|
| `CompetitionRules` enrichi (JSON) | `buildJSON()` (JS front) → body POST | handler `post_competition_rules` (deser) → use case `save_competition_rules` → repository (`serde_json` → JSONB) |
| `CompetitionRules` (lecture) | repository `find_rules` (`serde_json::from_str`) | récap phase-5 & summary_tab ; `existing_rules_json` → `initFromExistingRules()` (ré-hydratation front) |
| `Option<String>` (label bonus) | helper `format_bonus_label(&RankingRules)` | templates récap phase-5 & summary_tab |

## Cohérence des clés JSON (front ↔ domaine)

`initFromExistingRules()` et `buildJSON()` lisent/écrivent les clés serde du domaine.
Nouvelles clés à câbler côté JS :
- `defensive_bonus.max_td_conceded`
- `aggressive_bonus.activated` / `.points` / `.min_casualties`

## Règle métier à cette étape

Aucune nouvelle.