# Sous-total des joueurs disponibles

**Priorité : moyenne** — le chiffre existe dans l'en-tête de l'équipe, mais rien
ne permet de le vérifier ligne à ligne
**Périmètre : le widget joueurs du BC `players`**
**Dépend de :** rien
**Maquette :** `assets/rawpages/html/app-team-detail.html`

> **Carte révisée.** Écrite avant les cartes 488, 489, 492-495 et l'épic E15,
> elle décrivait un tableau à huit colonnes, un marquage des absents qui
> n'existait pas encore, et un booléen que le code a depuis appris à ne pas
> écrire. L'objectif, lui, n'a pas bougé — et la section « Ce qui a changé
> depuis » dit exactement quoi.

## Objectif

Une ligne de pied sous le tableau des joueurs :

```
7 joueurs disponibles — 1 absent, hors du compte      —      603 kPo
```

C'est ce qui compose la valeur d'équipe du prochain match, et le chiffre qu'un
coach vérifie avant de jouer.

## Ce qui a changé depuis la rédaction

### Déjà livré — la carte 489 a fait la moitié du travail

Le marquage des absents **existe**. `PlayerRowVm` porte `absence:
Option<Absence>`, la ligne porte `player-absent`, et une pastille affiche
l'icône et le libellé du motif. C'est plus riche que le `available: bool` prévu
ici, et il n'y a rien à refaire.

Le CSS correspondant vit dans `assets/static/css/widgets/players-widget.css`,
et non dans `pages/team-page.css` comme l'annonçait cette carte. Le pied s'y
ajoute — toujours aucune feuille neuve, rien à inscrire dans `css_bundle.rs`.

### Périmé — le tableau a treize colonnes, pas huit

Poignée de glisser-déposer, maillot, nom, poste, **cinq** caractéristiques,
compétences de base, compétences acquises, SPP, valeur. Le libellé du pied
prend donc `colspan="11"`, laissant SPP et Valeur seules à droite.

### Nouveau — les journaliers sont dans l'effectif

Depuis l'épic E15, un journalier est un vrai joueur et **apparaît dans la
liste**. Il entre dans le sous-total s'il est disponible, ce qui est cohérent :
la valeur d'équipe le compte aussi.

## Conception

### 1. Le prédicat vient du domaine — pas un booléen dans le VM

Cette carte proposait `pub available: bool` sur `PlayerRowVm`. **C'est
exactement ce que le code a appris à ne pas faire**, et `basket.rs` porte la
leçon :

> Un booléen `available_for_next_match` les confondait, et c'est ce qui a
> laissé les morts occuper une place : filtrer dessus aurait aussi libéré celle
> d'un blessé, qui revient au match suivant (BR12).

La règle « un view model transpose, il ne dérive pas » — postérieure à cette
carte — dit la même chose autrement : si le chiffre s'avérait faux, c'est le
domaine qu'on corrigerait, donc c'est de lui qu'il doit venir.

Or le prédicat n'existe pas. `PlayerParticipationStatus` (`Available`,
`MissingNextGame`, `Retired`, `Dead`) n'a **aucune méthode**, et deux endroits
comparent aujourd'hui des chaînes littérales à la main :
`infrastructure/teams/squad_adapter.rs` et `Absence::depuis_le_statut`.

```rust
// players/domain/match_impact.rs
impl PlayerParticipationStatus {
    pub fn from_str(valeur: &str) -> Self { … }

    /// Jouera-t-il le prochain match ?
    pub fn disponible(&self) -> bool { matches!(self, Self::Available) }
}
```

**Le défaut de `from_str` est prudent, et ce n'est pas un détail.** Sur le
modèle de `RosterMembership::from_str`, la tentation est un `_ => Available`.
Ici il serait dangereux dans le mauvais sens : un statut inconnu compterait
comme disponible et **gonflerait** un total que le coach croit vérifié. Seul
`"Available"` est reconnu comme disponible ; tout le reste ne compte pas —
c'est déjà ce que fait `squad_adapter`.

### 2. La même définition que la valeur d'équipe, ou rien

La valeur d'équipe ne retient que les alignables — `players_value` filtre sur
`SquadPresence::alignable()`, et l'adapter traduit `"Available"` et lui seul.

Le sous-total existe **pour rendre ce chiffre vérifiable**. S'il comptait un
joueur de plus ou de moins, il ne le vérifierait pas : il le contredirait. Les
deux prédicats doivent donc dire la même chose, et c'est la raison pour
laquelle celui-ci est posé dans le domaine plutôt que dans le widget.

### 3. Le sous-total, calculé au builder

```rust
pub struct PlayerTableTemplate {
    …,
    pub available_count: usize,
    pub unavailable_count: usize,
    pub available_value_kpo: i32,
}
```

Trois nombres plutôt qu'une structure : le gabarit les affiche, il ne les
manipule pas.

**`unavailable_count` sert à la mention « 1 absent, hors du compte ».** Elle
n'apparaît que s'il y en a — sinon la phrase serait un bruit permanent pour un
cas rare.

### 4. Le gabarit

Un `<tfoot>`, **pas une `<tr>` de fin de `<tbody>`** : le pied reste attaché au
tableau si la liste défile un jour, et il ne prend ni le zébrage
(`tr:nth-child(even)`) ni le survol des lignes de joueur.

Treize colonnes comme l'en-tête — mais **une cellule de poignée en tête, puis
`colspan="10"`**, et non un `colspan="11"` en première position.

`drag-handle-cell` est `display: none` hors mode édition et `table-cell`
dedans : la grille a **douze colonnes visibles en lecture et treize en
édition**. Un `colspan` fixe ne peut donc pas convenir aux deux. Le pied
commence comme une ligne du corps, et suit le même masquage.

**La colonne SPP porte un tiret, pas un total.** Les SPP ne s'additionnent pas
entre joueurs — chacun dépense les siens — et une somme y serait un nombre sans
signification.

**Pas de pied sur un effectif vide** : le gabarit rend déjà
`players-widget-empty` à la place du tableau, donc le cas se règle tout seul.

## Ce que la carte ne fait pas

- **Elle ne scinde pas le tableau.** Les absents restent dans la liste, à leur
  place, simplement marqués et hors du compte.
- **Elle ne touche pas à la valeur d'équipe.** Le sous-total est un affichage ;
  `team_value.rs` garde son calcul.
- **Elle ne refait pas `squad_adapter`.** Il continue de comparer ses chaînes ;
  le prédicat neuf est disponible pour lui, l'y brancher est un autre geste.
- **Elle n'explique pas l'absence.** Ni séquelle, ni blessure, ni durée : la
  pastille donne le motif, la fiche du joueur le détail.

## L'écart avec la valeur d'équipe

Le sous-total et la valeur d'équipe de l'en-tête **ne seront pas égaux**. Quatre
raisons, et la dernière manquait à cette carte :

| | |
|---|---|
| le staff | compte dans la VE, absent du tableau |
| les relances | idem |
| les journaliers **manquants** | comptés jusqu'à onze, sans ligne dans le tableau |
| **`LOW_COST_LINEMEN`** | un lineman n'y compte que ce qu'il a gagné **au-delà** de son prix |

La dernière est la plus visible : sur ces rosters, le sous-total dépassera
franchement la contribution réelle de l'effectif.

Le libellé « joueurs disponibles » dit déjà que c'est un sous-total de joueurs
et non une valeur d'équipe — mais c'est le genre d'écart qui remonte en
question, et il vaut mieux l'avoir prévu.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `le_sous_total_exclut_les_indisponibles` | le cœur |
| `sans_indisponible_la_mention_n_apparait_pas` | le cas le plus fréquent |
| `un_statut_inconnu_ne_compte_pas_comme_disponible` | le sens du défaut de `from_str` |
| `le_predicat_dit_la_meme_chose_que_la_valeur_d_equipe` | les deux définitions ne divergent pas |
| `un_journalier_disponible_entre_dans_le_compte` | E15 |
| **E2E** : le pied affiche le compte et la somme justes | bout en bout |
| **E2E** : le tiret et le total sont **sous leurs colonnes** | l'alignement, qu'aucun compte de cellules ne prouve |

## Checklist

- [x] `from_str` et `disponible()` sur `PlayerParticipationStatus`
- [x] Les trois nombres sur `PlayerTableTemplate`
- [x] Le `<tfoot>`, treize colonnes, `colspan="11"`, tiret sur SPP
- [x] Les styles dans `widgets/players-widget.css`
- [x] Les cinq tests unitaires et le test e2e
- [x] `make lint && make test && make check-arch && make e2e`

`make test` : 1734 passés. `make e2e` : 372 passés, 7 sautés.

## Ce qui a été fait

```
11 joueurs disponibles                              —      495 kPo
10 joueurs disponibles · 1 absent, hors du compte   —      795 kPo
```

Les deux relevés sont recoupés avec la base : `count(*)` et `sum(value_kpo)`
sur les seuls `Available` rendent bien `11|495` et `10|795`.

### Le VM transporte le statut, il ne tranche pas

`PlayerRowVm` porte `participation: PlayerParticipationStatus` — la donnée du
domaine — et non le `available: bool` que cette carte proposait. Le gabarit n'y
touche pas ; seul le constructeur pose la question à `disponible()`.

Le calcul vit dans `PlayerTableTemplate::new`, pas aux trois sites qui rendent
ce tableau : deux servent l'édition de l'effectif, où un compte divergent
serait invisible à la relecture.

### Deux défauts opposés, tous deux justes

C'est la trouvaille de la carte, et elle est consignée dans un test pour ne pas
être « corrigée » par mégarde :

| | devant un statut inconnu | pourquoi |
|---|---|---|
| l'affichage (carte 489) | **échoue ouvert** — ne barre rien | barrer ferait disparaître un effectif entier sur une faute de frappe |
| le compte (cette carte) | **échoue fermé** — ne compte pas | un total gonflé passerait pour juste, alors qu'on le regarde pour en vérifier un autre |

Un joueur peut donc être affiché sans repère **et** hors du compte. C'est le
moindre mal des deux côtés.

### Le pied porte les classes des colonnes qu'il totalise

`player-spp` et `player-value`, et non les `player-col-num` de l'en-tête : les
cellules du corps portent les premières, et un changement d'alignement de
colonne doit emmener le pied avec lui. Écrit d'abord avec les secondes, il
tombait juste par coïncidence — les deux valent `text-align: center`.

### L'e2e vit dans le fichier du blessé

`test_player_availability_after_injury.py` a déjà un fixture qui produit
exactement l'état voulu : une équipe dont un joueur, et un seul, manque le
prochain match. Lui en refaire un aurait coûté un second parcours complet pour
le même résultat.

Surtout, **le marquage et le sous-total sont une seule fonctionnalité** : un
total qui exclut une ligne sans dire laquelle paraîtrait faux, et un marquage
sans total n'a pas de conséquence. Le test les lit ensemble, et recoupe la
somme affichée sur les lignes non barrées — la vérification que le coach fait
de l'œil.

**Vu mordre** : le prédicat neutralisé, il rend
`'10 joueurs disponibles' in '11 joueurs disponibles'`.

### Le décalage de colonne, et pourquoi les tests ne l'ont pas vu

Première version livrée avec un `colspan="11"` : **le total tombait une colonne
trop à droite, hors du tableau**. Signalé à l'œil par l'utilisateur, pas par la
suite.

La cause est `drag-handle-cell`, `display: none` hors mode édition : la grille
a douze colonnes visibles en lecture et treize en édition, et aucun `colspan`
fixe ne convient aux deux. Le pied porte donc lui aussi une cellule de poignée,
puis `colspan="10"` — il suit le masquage comme les lignes du corps.

**Trois vérifications l'avaient laissé passer, et c'est ce qu'il faut en
retenir** : `check-arch` ne lit pas les gabarits ; les tests unitaires portaient
sur les nombres, qui étaient justes ; le test e2e lisait les libellés et
recoupait la somme — tous les chiffres étaient bons, seule leur **position**
était fausse. Compter les cellules du HTML ne l'aurait pas vu non plus : la
structure faisait bien treize colonnes des deux côtés, c'est le CSS qui en
retirait une.

Le test e2e mesure maintenant `getBoundingClientRect().x` du tiret et du total
contre celui des en-têtes SPP et Valeur. Vu mordre, le `colspan` fautif remis :
`{'spp': 1075, 'valeur': 1124, 'dash': 1124, 'total': 1197}`.

C'est le même enseignement que la carte 487, déjà cité dans ce fichier de
tests : **une classe posée ne prouve pas qu'une règle s'applique**, et une
structure correcte ne prouve pas un rendu correct. Ce qui se voit à l'œil se
teste en mesurant.
