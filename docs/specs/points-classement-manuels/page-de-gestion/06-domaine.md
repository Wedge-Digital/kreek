# Points de classement manuels · Phase 6 : domaine

**Phase 5** : `05-use-cases.md`

## Les règles, récapitulées — douze

### Ce que porte un point manuel

| | Règle |
|---|---|
| P1 | Il est **signé** — une pénalité est un point manuel négatif, pas une autre nature de chose |
| P2 | **Zéro est refusé** : une ligne qui ne change rien au classement est du bruit |
| P3 | Bornes **±100** — garde-fou contre la faute de frappe, jamais une règle du jeu |
| P4 | Le motif est **facultatif**, borné à 200 caractères, soumis à `TEXTE_SAISI` |
| P5 | Une ligne **se supprime, ne se modifie pas** |
| P6 | Deux lignes identiques sont **légitimes** — deux décisions, deux motifs |

### Ce qu'il fait au classement

| | Règle |
|---|---|
| C1 | Il entre dans le total **avant les départages** |
| C2 | Il **survit au rejeu** — il vit hors du cumul de `ranking_lines` |
| C3 | Le total d'une équipe **peut devenir négatif** |
| C4 | Il est **public**, et le classement l'affiche dans ses deux vues |

### Qui, et sur quoi

| | Règle |
|---|---|
| A1 | Seul un **admin de compétition ou d'espace** attribue et retire |
| A2 | Une équipe **non inscrite** à la saison ne reçoit rien |

---

## Ce que le domaine reçoit — trois touches

`ranking/domain/standings.rs` porte déjà tout ce qui ordonne un classement :
`TeamStanding`, `compare`, `order_standings`, `assign_ranks`,
`tiebreak_outcomes`. Cette fonctionnalité n'y ajoute aucune structure.

### 1 · `TeamStanding` gagne un champ, et il reste séparé

```rust
pub struct TeamStanding {
    pub team_id: TeamId,
    pub totals: CumulativeTotals,
    pub manual_points: i32,          // ← nouveau
}

impl TeamStanding {
    /// Le total qui ordonne et qui s'affiche : cumuls de match + points manuels.
    pub fn total_points(&self) -> i32 {
        self.totals.ranking_points.0 as i32 + self.manual_points
    }
}
```

**Ne pas additionner dans `totals`.** Celui-ci décrit fidèlement ce qui est
stocké dans `ranking_lines` ; y fondre les points manuels ferait mentir un type
sur sa source, et rendrait la décomposition impossible à retrouver — or la
colonne « Man. » du classement en dépend (C4).

### 2 · `compare` change d'une ligne, et c'est toute la règle C1

```rust
// aujourd'hui — standings.rs:51
b.totals.ranking_points.0
    .cmp(&a.totals.ranking_points.0)
    .then_with(|| …les critères de départage…)

// demain
b.total_points()
    .cmp(&a.total_points())
    .then_with(|| …les critères de départage…)
```

**Le commentaire de la fonction reste vrai mot pour mot :**

> Règle 18 : les points de classement d'abord, puis chaque critère actif dans
> l'ordre jusqu'au premier qui départage.

Ce sont les *points de classement* qui changent de définition, pas l'ordre des
opérations. C'est le signe que la règle s'insère à l'endroit prévu pour elle,
et non à côté.

**Ce que cette seule expression garantit** : une équipe à 3 points + 2 manuels
compare **égal** à une équipe à 5 sans manuel. Le `then_with` s'exécute alors,
et ce sont les départages qui tranchent. C1 tient là, et nulle part ailleurs.

`assign_ranks` et `tiebreak_outcomes` appellent `compare` : ils suivent sans
être touchés. **L'ex æquo, les rangs partagés et le marquage des cellules
décisives continuent de fonctionner** sur le total ajusté, gratuitement.

### 3 · Deux value objects

```rust
// ranking/domain/manual_points.rs
#[nutype(
    validate(predicate = |n| *n != 0 && n.abs() <= 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct ManualPoints(i32);          // P1, P2, P3

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 200, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct ManualPointsReason(String); // P4
```

**`TEXTE_SAISI` et non un charset propre.** C'est la constante unique de
`shared_kernel/identity/charset.rs`. Onze charsets ont coexisté dans ce projet,
dont neuf refusaient l'apostrophe — une compétence « Capitaine d'équipe »
échouait sur un `UnknownSkill` qui accusait le catalogue. On n'en rouvre pas un
douzième.

**200 caractères et non 50** : un motif est une phrase — « Forfait des Griffons
d'Argent à la journée 3, rencontre non jouée » — là où un nom est un libellé.

## Le seul entier nu du domaine, et pourquoi

`total_points()` rend un `i32`, quand tout le reste manipule des `u32` bornés
par nutype.

C'est délibéré. Le total **peut être négatif** (C3) et `RankingPoints` est un
`u32` : il ne peut pas le porter. Lui créer un frère signé —
`AdjustedPoints(i32)` — ajouterait un type dont la seule fonction serait
d'envelopper une addition, sans invariant à garder : **toute** valeur signée est
un total valide.

Un value object qui ne refuse rien n'est pas un value object, c'est une
cérémonie.

`manual_points: i32` dans `TeamStanding` suit la même logique : le champ reçoit
une somme de `ManualPoints` déjà validés, et une somme de valeurs valides est
valide.

## Ce que le domaine ne fait pas

- **Aucun agrégat, aucune méthode de mutation.** Un point manuel n'a pas de
  cycle de vie : il est écrit, lu, supprimé.
- **Aucun contrôle d'autorisation** (A1) ni d'inscription (A2) — les deux
  demandent des ports, que le `CLAUDE.md` interdit au domaine. Ils vivent dans
  les use cases (phase 5).
- **Aucune variante de `DomainError`.** Les deux refus — zéro et hors bornes —
  sont portés par les nutypes, qui échouent à la construction.

## Tests

### Les value objects

| Test | Règle |
|---|---|
| `manual_points_refuse_zero` | P2 |
| `manual_points_accepte_un_negatif` | P1 |
| `manual_points_refuse_au_dela_de_cent` | P3, aux deux extrémités |
| `reason_refuse_le_vide_apres_trim` | P4 |
| `reason_accepte_une_apostrophe` | le piège du charset, explicitement |

`reason_accepte_une_apostrophe` n'est pas décoratif : c'est le test qui aurait
attrapé les neuf charsets fautifs.

### L'ordre du classement — le cœur

| Test | Règle |
|---|---|
| `un_point_manuel_entre_dans_le_total` | C1 |
| **`trois_plus_deux_manuels_egale_cinq_sans_manuel`** | C1, littéralement |
| `l_egalite_ainsi_creee_est_tranchee_par_les_departages` | C1 — le `then_with` s'exécute |
| `un_point_manuel_negatif_fait_descendre` | C3 |
| `un_total_negatif_est_classe_normalement` | C3 |
| `sans_point_manuel_le_classement_est_inchange` | la non-régression |

**`trois_plus_deux_manuels_egale_cinq_sans_manuel` est le test de cette
fonctionnalité.** Il énonce la règle telle qu'elle a été posée, et il échoue si
quelqu'un déplace un jour l'addition après le tri — ce qui compilerait
parfaitement.

**`sans_point_manuel_le_classement_est_inchange`** est son pendant : il garantit
que les saisons qui n'utilisent pas la fonctionnalité s'ordonnent exactement
comme avant.

## Règles métier

**Aucune à préciser.** Les douze couvrent la fonctionnalité, et cette phase
n'en fait apparaître aucune — elle montre au contraire que la plus structurante
tient à une seule expression, à l'endroit que le domaine prévoyait déjà.
