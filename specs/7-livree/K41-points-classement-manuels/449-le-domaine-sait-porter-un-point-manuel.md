# Le domaine sait porter un point manuel

**Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/points-classement-manuels/page-de-gestion/06-domaine.md`

## Objectif

Faire entrer les points manuels dans le calcul du classement, **avant les
départages**. Aucun écran, aucune table, aucune route.

## Pourquoi elle est seule

Elle touche `compare`, la fonction qui ordonne **tous** les classements de
l'application. Une régression là-dessus se voit sur chaque compétition, et elle
doit se relire sans être mêlée à une table neuve.

## Conception

### 1. Deux value objects

```rust
// ranking/domain/manual_points.rs
#[nutype(
    validate(predicate = |n| *n != 0 && n.abs() <= 100),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct ManualPoints(i32);

#[nutype(
    sanitize(trim),
    validate(not_empty, len_char_max = 200, regex = TEXTE_SAISI),
    derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, AsRef)
)]
pub struct ManualPointsReason(String);
```

**Signé** : une pénalité est un point manuel négatif, pas une autre nature de
chose.

**Zéro refusé** : une ligne qui ne change rien au classement mais occupe le
relevé et demande à être expliquée est du bruit.

**±100** est un garde-fou contre la faute de frappe — un `300` pour un `3` — et
non une règle du jeu. Si une ligue a besoin de davantage, c'est la borne qui
bouge, pas le principe.

**`TEXTE_SAISI` et non un charset propre.** Onze charsets ont coexisté dans ce
projet, dont neuf refusaient l'apostrophe. On n'en rouvre pas un douzième.
**200 caractères** parce qu'un motif est une phrase, pas un libellé.

### 2. `TeamStanding` gagne un champ, et il reste séparé

```rust
pub struct TeamStanding {
    pub team_id: TeamId,
    pub totals: CumulativeTotals,
    pub manual_points: i32,          // ← nouveau
}

impl TeamStanding {
    pub fn total_points(&self) -> i32 {
        self.totals.ranking_points.0 as i32 + self.manual_points
    }
}
```

**Ne pas additionner dans `totals`** : celui-ci décrit fidèlement ce qui est
stocké dans `ranking_lines`. Y fondre les points manuels ferait mentir un type
sur sa source, et rendrait impossible la décomposition dont la colonne du
classement dépend.

**`i32` et non un value object.** Le total peut être négatif, `RankingPoints`
est un `u32`, et lui créer un frère signé ajouterait un type **sans invariant à
garder** — toute valeur signée est un total valide. Un value object qui ne
refuse rien est une cérémonie.

### 3. La ligne qui porte toute la règle

```rust
// standings.rs:51 — aujourd'hui
b.totals.ranking_points.0.cmp(&a.totals.ranking_points.0).then_with(|| …)

// demain
b.total_points().cmp(&a.total_points()).then_with(|| …)
```

**Le commentaire de `compare` reste vrai mot pour mot** — « les points de
classement d'abord, puis chaque critère actif ». Ce sont les points qui changent
de définition, pas l'ordre des opérations.

Une équipe à 3 + 2 manuels compare **égal** à une équipe à 5 sans manuel : le
`then_with` s'exécute, et les départages tranchent.

**`assign_ranks` et `tiebreak_outcomes` appellent `compare`** : l'ex æquo, les
rangs partagés et le marquage des cellules décisives suivent sans être touchés.

### 4. La signature du service

```rust
pub fn build_ordered_standings(
    lines: Vec<RankingLineRow>,
    manual: &HashMap<String, i32>,   // ← nouveau
    order: &TiebreakOrder,
) -> Vec<(TeamStanding, Rank)>
```

Les deux appelants — `classement_widget` et `detailed_standings_widget` —
passeront une carte vide jusqu'à la carte 451.

## Tests

### Les value objects

| Test | Règle |
|---|---|
| `manual_points_refuse_zero` | une ligne nulle est du bruit |
| `manual_points_accepte_un_negatif` | le signe porte le sens |
| `manual_points_refuse_au_dela_de_cent` | aux **deux** extrémités |
| `reason_refuse_le_vide_apres_trim` | un motif blanc n'est pas un motif |
| `reason_accepte_une_apostrophe` | le piège des neuf charsets fautifs |

### L'ordre du classement

| Test | Règle |
|---|---|
| `un_point_manuel_entre_dans_le_total` | l'addition a lieu |
| **`trois_plus_deux_manuels_egale_cinq_sans_manuel`** | la règle, littéralement |
| `l_egalite_ainsi_creee_est_tranchee_par_les_departages` | le `then_with` s'exécute |
| `un_point_manuel_negatif_fait_descendre` | le sens négatif |
| `un_total_negatif_est_classe_normalement` | un total à −2 est valide |
| `sans_point_manuel_le_classement_est_inchange` | **la non-régression** |

`trois_plus_deux_manuels_egale_cinq_sans_manuel` échoue si quelqu'un déplace un
jour l'addition **après** le tri — ce qui compilerait parfaitement.

`sans_point_manuel_le_classement_est_inchange` garantit que les saisons qui
n'usent pas de la fonctionnalité s'ordonnent exactement comme avant.

## Checklist

- [x] `ranking/domain/manual_points.rs` et les cinq tests — **six**, cf. ci-dessous
- [x] `TeamStanding.manual_points` et `total_points()`
- [x] La ligne de `compare`, et les six tests d'ordre
- [x] `build_ordered_standings` prend la carte, les deux appelants passent vide
- [x] `make lint && make test && make check-arch`

## Trois tests que la carte ne prévoyait pas

### La carte doit être **lue**, et rien ne le vérifiait

Les deux appelants passent une carte vide jusqu'à la 451. `to_standing` pouvait
donc ignorer son argument et poser zéro **sans qu'aucun test ne bronche** — et
la carte 451 aurait cherché longtemps pourquoi ses points n'apparaissaient pas,
avec un domaine correct et un service muet.

`la_carte_des_points_manuels_atteint_le_classement` ferme ce trou, et
`les_points_manuels_changent_les_rangs_attribues` vérifie que la lecture a lieu
**avant** l'attribution des rangs — pas seulement que le champ est rempli.

### Le sixième test de value object

`reason_refuse_au_dela_de_deux_cents_caracteres` : la carte listait cinq tests et
citait la borne sans la couvrir.

### Une équipe absente vaut zéro, et c'est la règle

`une_equipe_absente_de_la_carte_a_zero_point_manuel` : la carte ne porte que les
équipes qui ont reçu des points, elle n'est donc **jamais complète**. Le
`unwrap_or(0)` n'est pas un repli sur erreur mais le cas commun — c'est une
distinction qui se perd vite à la relecture.

## Falsification

| Mutation | Constaté |
|---|---|
| `compare` revient aux seuls `ranking_points` — *l'addition déplacée après le tri* | 4 rouges ; **`sans_point_manuel_le_classement_est_inchange` reste vert** |
| `to_standing` ignore la carte | 2 rouges, les deux tests de service ajoutés |
| `total_points()` soustrait au lieu d'additionner | 6 rouges |
| La borne ±100 retirée | 1 rouge, `manual_points_refuse_au_dela_de_cent` |

La première ligne est la plus instructive : la non-régression **reste verte**
sous la mutation qui casse la fonctionnalité. C'est ce qu'on attend d'elle — elle
répond « les saisons sans points manuels s'ordonnent comme avant », pas « la
fonctionnalité marche ». Les deux questions ont chacune leurs tests.

## Ce qui reste volontairement mort

`build_ordered_standings` prend un paramètre que ses deux appelants passent vide.
C'est une couture assumée : la carte 451 ne changera que les appelants, sans
retoucher `to_standing`. Signalé ici pour qui s'arrêterait dessus.

## Vérification supplémentaire

La carte n'expose rien et ne prescrit pas d'e2e. La suite complète a tout de
même été passée : `compare` ordonne **tous** les classements de l'application, et
un test unitaire de non-régression ne dit rien de ce que les pages affichent.
