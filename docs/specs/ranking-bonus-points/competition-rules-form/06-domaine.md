# Phase 6 — Domaine (competition-rules-form)

## Récapitulatif exhaustif des règles métier (validé)

**Règles de calcul** (implémentées dans l'unité `post-match-bonus-calc` — rappelées
ici pour complétude) :

1. Points finaux d'une équipe sur un match = points V/N/D **+** somme des bonus
   actifs gagnés.
2. Un bonus n'est calculé que si `activated == true` pour la compétition (sinon 0).
3. **Offensif** : +X pts si TD **marqués** par l'équipe **≥ seuil** (`min_td`).
4. **Défensif** : +X pts si TD **encaissés** par l'équipe **≤ seuil**
   (`max_td_conceded`).
5. **Agressif** : +X pts si **sorties** (`Sortie` seule) infligées à l'adversaire
   **> Y** (`min_casualties`, strict).
6. Les 3 bonus sont **cumulables** et **indépendants du résultat** (une équipe qui
   perd peut les toucher).
7. Évaluation **par équipe**, sur le match courant.
8. Chaque bonus est **autonome** : aucune contrainte inter-champs ni dépendance au
   barème V/N/D.

**Portée domaine de cette unité** : uniquement la **définition/validation** des
règles (value objects + structs). La logique de calcul (règles 1-7) vit dans
`post-match-bonus-calc`.

## Value objects (nutype, smart constructors)

| VO | Rôle | Validation | Défaut (rétro-compat) |
|---|---|---|---|
| `MinTd(u32)` | seuil TD marqués (offensif) — **renommé depuis `TdDiff`** | `1..=16` | — (champ existant) |
| `MaxTdConceded(u32)` | seuil TD encaissés (défensif) | `<= 16` (0 autorisé = shutout) | `1` |
| `MinCasualties(u32)` | seuil Y sorties (agressif) | `<= 16` (0 autorisé) | `2` |
| `RankingPoints(u32)` | points de bonus X | existant `<= 100_000` | — |
| `Activated(bool)` | flag d'activation | existant | `false` (agressif) |

### Renommage `TdDiff` → `MinTd`

Sémantique validée = « TD marqués ≥ seuil » (pas un écart). Renommage du VO et du
champ pour lever l'ambiguïté (le nom `diff_td` reste par ailleurs utilisé, à juste
titre, par le **critère de départage** `diff_td` = vraie différence de TD, concept
distinct).

- VO `TdDiff` → `MinTd`, bornes inchangées `1..=16`.
- Champ `OffensiveBonus.diff_td` → `OffensiveBonus.min_td`, avec
  **`#[serde(rename = "diff_td")]`** : la clé JSON reste `diff_td`.
- **Impact nul** sur la rétro-compat JSONB et sur le JS front (input `off_diff_td`,
  clé JSON `diff_td` inchangés).
- Impact Rust : les accès `rr.offensive_bonus.diff_td` deviennent
  `rr.offensive_bonus.min_td` (récap phase-5 & summary_tab, via le futur helper
  `format_bonus_label`).

## Structs domaine (competition_rules.rs)

```rust
pub struct OffensiveBonus {
    pub activated: Activated,
    #[serde(rename = "diff_td")]
    pub min_td: MinTd,            // renommé
    pub points: RankingPoints,
}

pub struct DefensiveBonus {
    pub activated: Activated,
    pub points: RankingPoints,
    #[serde(default = "default_max_td_conceded")]
    pub max_td_conceded: MaxTdConceded,   // nouveau
}

pub struct AggressiveBonus {              // nouvelle struct
    pub activated: Activated,
    pub points: RankingPoints,
    pub min_casualties: MinCasualties,
}

pub struct RankingRules {
    // … existants …
    pub offensive_bonus: OffensiveBonus,
    pub defensive_bonus: DefensiveBonus,
    #[serde(default = "default_aggressive_bonus")]
    pub aggressive_bonus: AggressiveBonus,   // nouveau
    pub additionnal_ranking_points: HashMap<String, u32>,
}
```

Fonctions de défaut (rétro-compat désérialisation) :

```rust
fn default_max_td_conceded() -> MaxTdConceded { MaxTdConceded::try_new(1).unwrap() }
fn default_aggressive_bonus() -> AggressiveBonus {
    AggressiveBonus {
        activated: Activated(false),
        points: RankingPoints::try_new(1).unwrap(),
        min_casualties: MinCasualties::try_new(2).unwrap(),
    }
}
```

## Erreurs domaine

Aucune nouvelle variante `DomainError` : les nutype renvoient leur propre erreur de
validation à la désérialisation / construction. Pas de règle inter-champs à
arbitrer côté domaine (bonus autonomes).

## Tests unitaires prévus (co-localisés)

1. `MaxTdConceded` / `MinCasualties` : construction valide (0, 16) et rejet (17).
2. `MinTd` : bornes inchangées (1 ok, 0 rejet, 16 ok, 17 rejet).
3. **Désérialisation legacy** : un JSON `ranking_rules` sans `max_td_conceded` ni
   `aggressive_bonus` → défauts appliqués (`max_td_conceded=1`, agressif désactivé).
4. **Compat clé renommée** : un JSON avec `offensive_bonus.diff_td` désérialise bien
   dans le champ `min_td`.
5. Round-trip serde : sérialisation → désérialisation conserve les valeurs (dont la
   clé JSON `diff_td`).

## Règle métier — question de clôture

Voici les règles identifiées (§ récapitulatif). **Règles manquantes ou à corriger ?**
→ À valider avec l'utilisateur avant Phase 7.
