# Le classement affiche les points manuels

**Ordre :** 2 · **Dépend de :** 449, 450 · **Prérequis : la carte 448**
**Conception :** `docs/specs/points-classement-manuels/page-de-gestion/`
(`04-dtos.md`, `07-integration.md`) · **Maquette :**
`assets/rawpages/html/app-competition-detail.html`

## Objectif

Rendre les points manuels visibles dans les deux vues du classement, et les
faire entrer dans l'ordre.

## La carte 448 passe avant, ou avec

`widgets/ranking-detailed-standings-widget.css` met le zébrage en `--dark-7`
(ligne 40) et le survol en `--dark-6` (ligne 44) — **deux valeurs séparées par
un rapport de contraste de 1,0012**, c'est-à-dire indiscernables.

**Le survol du classement détaillé est invisible une ligne sur deux, en
production, aujourd'hui.**

Ajouter une colonne à ce tableau sans corriger cela livrerait une nouveauté dans
un écran déjà cassé. Ce n'est pas un couplage de convenance : c'est le même
fichier, et la 451 n'a pas de sens sans elle.

## Conception

### Une lecture de plus, en parallèle

`classement_widget.rs:53` et `detailed_standings_widget.rs:96` ajoutent
`find_manual_totals_for_season` à leur `tokio::join!` existant :

```rust
let (rules, teams, lines, groups, manual) = tokio::join!(…);
…
build_ordered_standings(lines, &manual, &order)
```

**Quatre requêtes deviennent cinq, en parallèle** — le temps de réponse ne
bouge pas.

### Les deux view models

```rust
// classement_widget.rs
pub struct ClassementRowVm {
    …,
    pub points: u32,               // inchangé : les points de match
    pub manual: Option<i32>,       // ← None si l'équipe n'en a aucun
    pub total: i32,                // ← était u32
}

// detailed_standings_widget.rs
pub struct DetailedRowVm {
    …,
    pub bonus: String,             // inchangé, signé
    pub manual: Option<String>,    // ← « −1 », signé
    pub total: i32,                // ← était u32
}
```

**`Option` et non un zéro par convention.** Le gabarit doit distinguer « aucun
point manuel » — un tiret, non cliquable — de « zéro point manuel », **qui
n'existe pas** puisque `ManualPoints` le refuse. L'`Option` rend cette
impossibilité dans le type.

**`total` passe de `u32` à `i32`** : c'est le seul changement de type que la
fonctionnalité impose au code existant, et il découle d'un total qui peut
devenir négatif.

### Les deux colonnes

| Gabarit | Position | En-tête |
|---|---|---|
| `classement-widget.html` | entre `D` et `Pts` | « Man. » |
| `detailed-standings-widget.html` | entre `Bonus` et `Total` | « Manuel » |

**Dans le groupe « Points », jamais à côté des départages.** C'est la règle
rendue par la structure du tableau : les points manuels entrent dans le total
*avant* qu'on départage. Le `colspan` du groupe « Points » du détaillé passe de
2 à 3.

**Un point manuel non nul est un lien** vers la page de gestion — on voit un
chiffre qui surprend, on veut savoir d'où il vient. Le tiret d'un zéro n'en est
pas un.

### Le bouton d'accès

Dans les **deux onglets de classement**, et nulle part ailleurs. Il ne s'affiche
qu'aux administrateurs ; la page qu'il ouvre est consultable par tous.

**La duplication est délibérée, et le commentaire du gabarit doit le dire** : la
page de compétition compte six onglets, et au-dessus d'eux le bouton
s'afficherait sur Calendrier, Équipes et Statistiques, où il n'a rien à faire.
Sans ce commentaire, quelqu'un la « factorisera » dans six mois.

### Responsivité

Sous 768 px, **la colonne « Man. » reste**. C'est une composante du total, et la
masquer rendrait le total inexplicable — exactement ce que la fonctionnalité
cherche à réparer.

## Tests

Unitaires, sur les builders :

| Test | Ce qu'il prouve |
|---|---|
| `une_equipe_sans_point_manuel_rend_none` | le tiret, pas un zéro |
| `le_total_affiche_inclut_le_point_manuel` | l'addition arrive au VM |
| `un_total_negatif_se_rend_signe` | « −2 » |

Les tests de navigateur sont la carte 453.

## Checklist

- [x] **La carte 448 est passée** — commit `942a3c8`
- [x] Les deux `tokio::join!` et l'appel à `build_ordered_standings`
- [x] Les deux VM, `manual: Option`, `total: i32`
- [x] Les deux colonnes, `colspan` du détaillé corrigé
- [ ] ~~Le lien sur un point non nul~~ → **reporté en carte 452**
- [ ] ~~Le bouton dans les deux onglets~~ → **reporté en carte 452**
- [x] `make lint && make test && make check-arch`

## Trois écarts à la carte

### Le lien et le bouton partent en 452

La carte veut qu'un point manuel non nul soit un lien vers la page de gestion,
et un bouton d'accès dans les deux onglets. **Cette page n'existe pas encore** :
`ranking/routes.rs` ne porte aucune route de ce nom, et c'est la carte 452 qui
la livre. Les poser ici mettrait un lien mort en production entre les deux
commits.

Ils partent donc en 452, où leur cible existe. Le commentaire sur la duplication
délibérée du bouton — que la carte demande à juste titre, pour qu'on ne le
« factorise » pas au-dessus des six onglets — part avec eux.

### Le VM compact n'avait pas de `total`

La carte écrit `total: i32, // ← était u32`. `ClassementRowVm` portait
`points: u32` et rien d'autre : c'est `points` qui jouait le rôle de total. Le
VM porte désormais les trois — `points` (les seuls points de match, qui ne
s'affichent plus), `manual` et `total` —, et la colonne « Pts » montre `total`.
Le détaillé, lui, avait bien un `total: u32` devenu `i32`, comme annoncé.

### `Option<String>` et non `Option<i32>` pour le compact

Askama lie `m` en `&i32` dans un `if let`, et le comparer à un littéral
demanderait un déréférencement que les gabarits du projet n'emploient pas. Le
signe se pose donc dans le builder, une fois — l'idiome que `bonus` du classement
détaillé établissait déjà. L'`Option` reste, et c'est elle qui compte : elle
distingue « aucun » de « zéro », lequel n'existe pas puisque `ManualPoints` le
refuse.

## La légende devenait fausse

`detailed-standings-widget.html` annonçait « **Total** = points de
victoire/nul/défaite + bonus ». Avec une troisième composante, la phrase aurait
contredit la colonne voisine. Non mentionné par la carte ; étendu.

## Un test que la carte ne prévoyait pas, et qui a servi tout de suite

`les_deux_vues_s_accordent_sur_le_total` : les deux classements calculent leur
total par deux builders distincts, et **rien ne les comparait**. Une divergence
afficherait deux classements contradictoires sur la même page, chacun cohérent
avec lui-même.

Il a mordu à la première exécution : `build_detailed_groups` avait reçu le
paramètre `manual` **sans le faire descendre** jusqu'à `build_ordered_standings`.
Le classement compact était juste, le détaillé ignorait les points manuels, et
seul ce test le disait.

## Falsification

| Mutation | Constaté |
|---|---|
| `Some(0)` au lieu de `None` | `une_equipe_sans_point_manuel_rend_none` rouge |
| Le détaillé cesse de faire descendre la carte | 2 rouges, dont le test des deux vues |
| **Le gabarit affiche `row.points` au lieu de `row.total`** | **1503 tests verts** |

La troisième ligne dit la limite exacte de cette carte : un classement dont
l'affichage contredirait son propre ordre passerait toute la suite unitaire. Ce
défaut-là relève de la carte 453.

## Vérification à l'écran

Deux points manuels posés en base de développement — `+4` sur une équipe, `−3`
sur une autre :

```
compact   #  ÉQUIPE  MJ V N D  MAN.  PTS
          1  …                  +4    10
          2  …                  —      3
          3  …                  −3     0
          4  …                  —      0

détaillé  groupes : ['', 'MATCHS', 'POINTS', 'DÉPARTAGES…']
          en-têtes : … BONUS  MANUEL  TOTAL  1 · Δ TD …
```

L'ordre bouge réellement : l'équipe sanctionnée passe de deuxième *ex æquo* à
troisième.

Le bandeau de groupes et les colonnes s'accordent : somme des `colspan` = 16,
colonnes réelles = 16. Un `colspan` oublié aurait décalé tout le bloc des
départages sous les mauvais en-têtes.

## Un test e2e a payé le changement de colonne

`test_detailed_standings.py` portait `FIXED_COLUMNS = 8` — le nombre de colonnes
qui précèdent le bloc des départages. La neuvième colonne décalait la fenêtre de
`_tiebreak_headers`, qui rendait alors « Total » comme s'il s'agissait d'un
critère.

**Aucun test unitaire ne pouvait le voir** : la constante vit dans la suite
navigateur, et c'est elle qui a rougi. La constante porte désormais un
commentaire disant qu'elle doit suivre toute colonne ajoutée avant les
départages.
