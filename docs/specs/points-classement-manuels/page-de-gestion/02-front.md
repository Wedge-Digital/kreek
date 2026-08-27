# Points de classement manuels · Phase 2 : architecture front

**Maquettes** : `app-manual-ranking-points.html` et `app-competition-detail.html`

## Deux surfaces, pas une

La fonctionnalité touche **deux écrans qui ne sont pas dans le même BC** :

| Surface | BC | Ce qu'elle fait |
|---|---|---|
| La page de gestion | `ranking` | attribuer, lister, supprimer |
| Les deux onglets de classement | `ranking`, hébergés par `competitions` | afficher la colonne, ouvrir la page |

Le second point mérite d'être dit : les widgets de classement **appartiennent à
`ranking`** (`ranking/io/web/widgets/`), et la page de compétition ne fait que
les charger par `hx-get` — `competition-tab-standings.html:2`. Ajouter une
colonne ne touche donc pas `competitions`.

## La page de gestion — un assemblage à deux widgets

Elle a **deux sections qui mutent indépendamment** : le formulaire d'attribution
et la liste. Le `CLAUDE.md` réserve le patron d'assemblage à trois et plus, et
deux ne le justifient pas — mais elles communiquent, et c'est ce qui décide.

| Widget | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|
| `#mp-form` | `manual_points_form` | `load` | — | attribution |
| `#mp-list` | `manual_points_list` | `load`, et `manualPointsChanged from:body` | — | liste + suppression |

**Un seul événement DOM, et il va dans un seul sens.**

```
POST attribuer   → HX-Trigger: manualPointsChanged → #mp-list se recharge
DELETE une ligne → HX-Trigger: manualPointsChanged → #mp-list se recharge
```

Le formulaire, lui, **n'écoute rien** : rien de ce qui se passe dans la liste ne
change ce qu'il propose. La liste des équipes ne bouge pas, le sens et les
points non plus.

**Pourquoi pas un seul fragment.** Le formulaire garde son état — équipe
choisie, sens, points saisis — pendant qu'on attribue plusieurs lignes d'affilée
à des équipes différentes. Un fragment unique le réinitialiserait à chaque
enregistrement, et c'est précisément le geste répété qu'on veut servir : un
arbitre qui traite les forfaits d'une journée en attribue trois ou quatre à la
suite.

## Le formulaire

**`kreek-select` pour l'équipe**, alimenté par `url` — c'est le composant imposé
par le `CLAUDE.md`, et la liste des équipes inscrites est déjà servie par
`competitions`.

**Le sens est une bascule, pas un signe à saisir.** « + Bonus » / « − Pénalité ».
Un ajustement manuel est aussi souvent une sanction qu'une faveur, et un `-3`
tapé au clavier se tape aussi bien par erreur. Le champ Points ne prend que des
entiers positifs ; le handler compose.

**Le motif est facultatif** (tranché en phase 1). Le pied le rappelle sans
l'exiger.

### Le pied de conséquence

Il annonce l'effet **avant** qu'il ne se produise — « les Trolls du Bief
passeront de 5 à 8 points manuels ». État d'écran pur, dérivé des trois champs,
sans aller-retour.

C'est le mécanisme des paramètres de compétition : la place ne bouge pas, le ton
change.

## La liste

**Groupée par équipe, en accordéon, repliée par défaut.** Plié, un bloc donne le
total de l'équipe et son nombre de lignes ; déplié, le détail.

Quinze lignes à plat noieraient l'essentiel — combien chaque équipe a reçu. Le
groupement répond au « de chaque équipe » de la demande, les lignes au « liste
de ligne ».

**L'en-tête de colonnes vit dans chaque bloc**, replié avec lui : posé une seule
fois en haut, il surplomberait des sections fermées et ne désignerait rien.

**Une ligne se supprime, elle ne se modifie pas** (phase 1). Le `✕` est le geste
de retrait du reste du site — six occurrences dans les gabarits de production
contre une seule poubelle.

## Le classement — ce qui change

### La colonne, dans les deux vues

| Vue | Où | Forme |
|---|---|---|
| Classement simple | entre `D` et `Pts` | « Man. » |
| Classement détaillé | entre `Bonus` et `Total` | « Manuel » |

**Dans le groupe « Points », jamais à côté des départages.** C'est la règle 2
rendue par la structure du tableau : les points manuels entrent dans le total
*avant* qu'on départage. Le `colspan` du groupe « Points » passe donc de 2 à 3
dans le détaillé.

**Un point manuel non nul est un lien** vers les lignes de cette équipe. C'est
le geste réel : on voit un chiffre qui surprend, on veut savoir d'où il vient.
Le tiret d'un zéro n'est pas un lien — il n'y a rien à aller lire.

### Le bouton d'accès

**Répété dans les deux onglets de classement, et nulle part ailleurs.** La page
de compétition en compte six ; au-dessus des onglets, le bouton s'afficherait
sur Calendrier, Équipes et Statistiques, où il n'a rien à faire.

**La duplication est assumée** — c'est le prix de cette justesse, et le
commentaire du gabarit le dit pour que personne ne la « factorise » plus tard.

**Il ne s'affiche qu'aux administrateurs** ; la page qu'il ouvre est consultable
par tous.

## Ce qui reste front

| Front | Serveur |
|---|---|
| le pied de conséquence | la liste des équipes (`kreek-select` par `url`) |
| l'accordéon | tout le reste |
| la bascule de sens | |

**Aucun calcul de points côté client.** Le pied annonce, il ne décide pas.

## Responsivité

Desktop-first, breakpoint unique à 768 px.

Sur la page de gestion, la colonne **« Attribué par » disparaît en premier** :
c'est celle dont l'absence coûte le moins, le motif et la date restant lisibles.
Le formulaire passe en colonne.

Sur le classement, la colonne « Man. » **reste** : c'est une composante du
total, et la masquer rendrait le total inexplicable — exactement ce que la
fonctionnalité cherche à réparer.

## CSS

Deux feuilles, à inscrire dans `src/web/css_bundle.rs` — l'axe 14 refuse toute
feuille absente du bundle :

- `pages/ranking-manual-points.css`, portée par `.mp-page` ;
- les colonnes du classement s'ajoutent aux feuilles existantes
  `widgets/ranking-classement-widget.css` et
  `widgets/ranking-detailed-standings-widget.css`.

> **À faire au passage** : ces deux feuilles portent le défaut de la **carte
> 448** — `--dark-7` pour le zébrage, `--dark-6` pour le survol, deux valeurs
> qui ne se distinguent pas. Ajouter une colonne sans corriger ça livrerait une
> nouveauté dans un tableau dont le survol est déjà invisible une ligne sur deux.

## Règles métier

**Aucune à préciser.** Les six de la phase 1 couvrent la fonctionnalité.

### Tranché — où vivent les points manuels

**Dans `ranking`, et dans une table à part.**

Le BC est celui qui calcule le classement : y loger la donnée évite un port pour
lire ce dont le calcul a besoin à chaque ligne. Et une table distincte de
`ranking_lines` est ce qui les fait **survivre au rejeu** — la carte 418 rejoue
la saison depuis zéro à partir des lignes cumulatives et recalcule les points
par `record_match`. Tout ce qui vit dans ce cumul serait effacé au premier
changement de barème.

C'est aussi ce qui fait des points manuels une **troisième composante** du
total, à côté des points de match et des bonus, plutôt qu'un ajustement fondu
dedans : ils ont leur propre source, ils gardent leur propre colonne.

### Reste à trancher en phase 3

**Le classement se recalcule-t-il à l'attribution ?** L'ordre des équipes change
dès qu'un point est attribué. Reste à savoir si cela emprunte le mécanisme du
recalcul de barème (carte 422) ou s'il suffit de relire — les points manuels
n'étant pas dans le cumul, une simple relecture pourrait suffire. Question de
phase 3.
