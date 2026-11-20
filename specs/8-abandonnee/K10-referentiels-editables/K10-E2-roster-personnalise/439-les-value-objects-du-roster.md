# Les value objects du roster

**Épic :** E10 — Référentiels éditables · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/roster-personnalise/editeur-de-roster/`
(`04-dtos.md`, `06-domaine.md`)

## Objectif

Donner au projet de quoi dire « cette caractéristique est valide », et réunir en
un seul endroit les types qui décrivent un roster. Aucun écran, aucune écriture.

## 1. Six types neufs — les caractéristiques

**Aucune borne n'existe nulle part** pour MA, ST, AG, PA et AV. Le corpus est un
fichier de confiance ; personne n'a jamais tapé une caractéristique. L'éditeur
de roster sera le premier.

Le LRB les donne — *Livre de Règles Bonifiées Saison 3*, page 25 :

| | M | F | AG | CP | AR |
|---|---|---|---|---|---|
| Maximum | 9 | 8 | 1+ | 1+ | 11+ |
| Minimum | 1 | 1 | 6+ | 6+ | 3+ |

```rust
// shared_kernel/bloodbowl/roster.rs
#[nutype(validate(greater_or_equal = 1,  less_or_equal = 9),  …)] pub struct Movement(u8);
#[nutype(validate(greater_or_equal = 1,  less_or_equal = 8),  …)] pub struct Strength(u8);
#[nutype(validate(greater_or_equal = 1,  less_or_equal = 6),  …)] pub struct AgilityTarget(u8);
#[nutype(validate(greater_or_equal = 1,  less_or_equal = 6),  …)] pub struct PassingTarget(u8);
#[nutype(validate(greater_or_equal = 3,  less_or_equal = 11), …)] pub struct ArmourTarget(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RosterTier { One, Two, Three }   // "Tier 1" … "Tier 3" à la sérialisation
```

### Le piège du tableau, à ne pas transcrire à l'aveugle

**« Maximum » veut dire *le meilleur*, pas *le plus grand*.** Le meilleur AG est
`1+`, numériquement le plus **petit**. La meilleure armure est `11+`,
numériquement la plus **grande**. Recopier la colonne « Maximum » en
`less_or_equal` inverserait deux bornes sur cinq.

Les intervalles du bloc de code sont **numériques** : ce sont eux qui font foi.

### `RosterTier` n'est pas `TierName`

`TierName` existe (`shared_kernel/bloodbowl/tier.rs`) et nomme la **catégorie
d'une compétition** — « Débutants », « Confirmés ». Texte libre, choisi par un
organisateur.

Le tier d'un **roster** est un classement de puissance défini par le règlement.
Les confondre ferait qu'un roster prenne un tier « Débutants » que rien ne
saurait comparer.

## 2. Cinq types déplacés depuis `team_creation`

`team_creation/domain/roster.rs` les porte, avec les bonnes bornes :

| Type | Bornes |
|---|---|
| `RosterName`, `PlayerName` | `trim`, non vide, ≤ 50 car., `TEXTE_SAISI` |
| `PlayerPrice` | 1 à 300 |
| `PlayerMaxQuantity` | 1 à 16 |
| `RerollBasePrice` | 1 à 100 |
| `CrossLimitCount` | 1 à 16 |

Ils montent dans `shared_kernel::bloodbowl::roster`, et `team_creation` suit par
un `use`.

**Déplacement par copier-coller** — règle 5 du `CLAUDE.md`. Ne pas réécrire de
mémoire : les bornes sont exactes, une seule fausse et un roster valide devient
refusé.

### Pourquoi déplacer plutôt que redéclarer

`references` va en avoir besoin, et **n'a pas le droit d'importer
`team_creation`**. Redéclarer donnerait deux définitions de « ce qu'est un nom
de poste valide », qui dérivent — un roster accepté à l'écriture et refusé à la
lecture, escamoté sans un mot. C'est la carte 438.

Ces types décrivent **le jeu**, pas un cas d'usage. `shared_kernel::bloodbowl::`
existe pour ça, et `TEXTE_SAISI` y est déjà.

### Un homonyme à ne pas emporter

`RosterName` existe **aussi** dans `teams/domain/value_objects.rs`. Celui-là
nomme le roster **d'une équipe**, pas un roster de référence. **Il reste où il
est.** À vérifier avant de déplacer, pas à supposer.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `les_bornes_suivent_le_lrb` | les cinq caractéristiques, aux **deux** extrémités |
| `une_force_de_9_est_refusee` | la borne haute de F est 8 — c'est celle qu'on se trompe à poser à 7 |
| `une_armure_de_2_est_refusee` | AR commence à 3, pas à 1 |
| `un_mouvement_de_0_est_refuse` | la borne basse compte autant que la haute |
| `roster_tier_se_serialise_comme_le_corpus` | `"Tier 1"`, pas `"One"` |
| les tests déplacés de `team_creation` | inchangés, ils suivent leurs types |

## Checklist

- [ ] `shared_kernel/bloodbowl/roster.rs` avec les six types neufs
- [ ] Les cinq types déplacés par copier-coller, `team_creation` adapté
- [ ] `teams::RosterName` vérifié et **non** touché
- [ ] Les tests, déplacés et neufs
- [ ] `make lint && make test && make check-arch`
