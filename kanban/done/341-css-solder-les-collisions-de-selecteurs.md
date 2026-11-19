# CSS — donner sa portée à chaque feuille, à rendu constant

**Priorité : haute** — rien n'était cassé, mais tant que ces collisions
existaient la carte 342 était impossible, et c'est elle qui supprime le
clignotement au chargement des pages
**Dépend de :** rien
**Bloque :** carte 342 (fusion du CSS en fichier unique chargé dans le `<head>`)
**Fichiers :** les 46 feuilles de `assets/static/css/{pages,widgets}/`, les
composants `create-card` et `competition-card`, une trentaine de templates,
`scripts/check-css-collisions.sh` (nouveau), `scripts/scoper-css.py` (nouveau),
`tests/e2e/visual/` (nouveau)

## Le problème

Les feuilles CSS de l'application ont été écrites en **isolation totale** :
chaque page charge la sienne via un `<link>` placé dans son fragment, et deux
feuilles de page ne cohabitent jamais dans un même document.

Cette isolation a autorisé, sans que rien ne le signale, la réutilisation des
mêmes noms de classe avec des valeurs différentes. Tant que les feuilles restent
isolées, ces divergences sont invisibles. Dès qu'on les réunit dans un fichier
unique, la dernière chargée gagne — **pour toute l'application**. Le bundle ne
neutralise pas les collisions, il les active.

## L'inventaire était périmé, et de moitié

La carte annonçait « 79 sélecteurs divergents sur 62 feuilles », mesurés à sa
rédaction. Remesuré au premier jour du chantier :

| | Carte | Réalité |
|---|---|---|
| Feuilles CSS | 62 | **75** |
| Sélecteurs distincts | 2 390 | 2 213 |
| **Divergents** | **79** | **116** |

C'est la troisième carte de suite dont le décompte avait dérivé — après la 347
(56 commandes annoncées, 62 réelles) et la 348 (45 fonctions, 58). Une carte
compte ce qui était vrai le jour où elle a été écrite ; le premier geste du
chantier est de recompter.

## La convention retenue : le nom du fichier **est** le sélecteur

`pages/team-page.css` ⇒ toute règle sous `.team-page`.

La carte proposait de réutiliser la classe racine de chaque template, en tenant
la correspondance dans un tableau. Ce tableau aurait dérivé, et surtout rien
n'aurait vérifié qu'une feuille est scopée sous la bonne ancre. En faisant du
nom de fichier le sélecteur, la règle se vérifie en comparant un nom à un
préfixe — une ligne de script, aucune table à tenir.

Les feuilles ont donc été **renommées d'après la racine de leur template** quand
elle était unique, et les templates ont **reçu une classe nommée d'après leur
feuille** quand elle ne l'était pas.

## Les quatre régimes de racine

Ils sont apparus un par un, chacun révélé par une régression que le harnais a
nommée.

**Racine unique** — la feuille est renommée d'après elle, le template ne bouge
pas. `pages/app-team-detail.css` devient `pages/team-page.css`.

**Racine partagée** — cinq feuilles de rapport de match ouvrent toutes sur
`.mr-container`, six pages de création sur `.create-card`. Chaque template reçoit
alors une classe supplémentaire, nommée d'après sa feuille.

**Pas de racine du tout** — le template ouvre sur un `<div>` nu. Il en reçoit
une, sans style attaché.

**Des frères de premier niveau** — deux pages d'administration n'ont pas de
racine unique : leurs éléments de premier niveau sont côte à côte, et les
fragments HTMX atterrissent à côté du panneau principal, pas dedans. La portée
est posée sur **chaque frère**, plutôt que d'inventer un conteneur et de modifier
la structure du DOM pour les besoins du CSS.

## Les deux formes de sélecteur

Le scoper émet **`.portee .x` et `.portee.x`**. Sans la seconde, une feuille qui
style son propre élément racine cesse de s'appliquer dès que la portée est une
classe *ajoutée* à cet élément : `<div class="mr-container match-report-inducements">`
n'est pas *à l'intérieur* de `.match-report-inducements`, il l'est. Le sélecteur
descendant l'exclut, et la feuille globale reprend la main — mesuré, un
`padding-bottom` de 120 px retombé à 24 px.

Le cas n'apparaît pas tant que la portée **est** la classe racine ; il surgit dès
qu'une racine porte deux classes dont une seule est la portée.

## Les feuilles qui ne se scopent pas

**Déclarées globales dans leur en-tête**, par un commentaire `css:global` portant
son motif — jamais dans une liste tenue par le script :

- `pages/match-report-shared.css`, chargée par dix templates ;
- `pages/app-home.css`, chargée par trois pages sans lien entre elles.

**`components/` reste global — sauf critère.** Le lot 3 a fait émerger une règle
plus juste : **un composant se scope s'il a une racine et qu'il s'y tient.**
`create-card` remplit la condition et a été scopé ; `team-selection` déborde de
sa racine et reste global.

**Hors périmètre, déduit et non listé** : douze feuilles qu'aucun template de
`src/` ne charge — six ne servent qu'aux maquettes, six ne servent à rien. Le
script les compte et les nomme à chaque exécution : hors périmètre n'est pas hors
de vue.

## L'exception au rendu constant — deux valeurs, mesurées

La contrainte « aucune valeur calculée ne change » a tenu partout sauf ici, et
l'écart est de même nature dans les deux cas : une feuille de page portait une
règle que la cascade étouffait, et lui donner sa portée la rend effective.

| Page | Élément | Avant | Après | Ce qui étouffait la règle |
|---|---|---|---|---|
| Actions de match | `div.mr-card` | `gap: 24px` | `gap: 12px` | `match-report-shared.css` chargée une seconde fois, après la feuille de page, par un widget du BC `players` (carte 358) |
| Inducements | `div.mr-tab` | `padding: 8px 6px` | `padding: 8px 4px` | `widgets/inducement-selector.css` stylait des onglets situés hors de sa racine, et gagnait par l'ordre |

Dans les deux cas, la valeur « après » est celle que l'auteur a écrite. La valeur
« avant » est le produit d'un accident.

**Pourquoi assumer plutôt que préserver.** Préserver aurait demandé d'inscrire
dans les feuilles de page les valeurs que la cascade imposait, c'est-à-dire d'y
graver une intention que personne n'a eue.

## Le harnais — des styles calculés, pas des captures d'écran

La carte prescrivait des captures d'écran. Un tel harnais a été écrit, puis
mesuré : après avoir neutralisé six hôtes externes, attendu les polices et gelé
les animations, il variait encore **de 5 à 12 % d'un passage à l'autre sans
qu'aucun CSS n'ait changé**.

Le harnais compare donc des **styles calculés**. Ce n'est pas un repli : « valeur
calculée » est le terme du navigateur, et `getComputedStyle` est littéralement la
mesure que cette carte demande. Vérification faite avant de basculer — les
43 pages rendent un DOM **identique** d'un passage à l'autre sur 13 280
éléments : l'instabilité était dans la peinture, jamais dans la structure.

| | Résultat |
|---|---|
| Couverture | 43 pages × 2 largeurs, **46 feuilles sur 46**, mesurée et non supposée |
| Déterminisme | **0 écart sur 78 702 relevés** entre deux passages |
| Précision | un écart nomme la vue, l'élément et la propriété |

Trois de ses raffinements sont venus de faux positifs qu'il fallait comprendre
plutôt que contourner : les éléments qui ne rendent rien sont exclus du relevé et
ne consomment pas d'indice de chemin — sans quoi la carte 342, qui supprimera
146 `<link>`, l'aurait rendu inutilisable ; le contrôle de stabilité exige que le
nombre d'éléments cesse de bouger, un fragment en `hx-trigger="load"` pouvant
n'avoir pas encore émis sa requête ; et le relevé attend le serveur, que
`cargo watch` redémarre à **chaque** modification de CSS.

## Ce qui a servi de méthode

**Les grappes se calculent, elles ne se choisissent pas.** Deux feuilles qui se
rencontrent dans le DOM d'une même page doivent recevoir leur portée ensemble,
sinon la spécificité monte d'un seul côté et change le gagnant. La carte de
co-chargement vient des relevés du harnais.

**Le regroupement par co-chargement est conservateur ; l'intersection réelle des
sélecteurs ne l'est pas.** Vérifier laquelle des feuilles d'une composante partage
vraiment un sélecteur a permis, deux fois, de sortir une feuille d'une grappe et
d'éviter un travail inutile.

**Quand une feuille déborde de sa racine, on copie plutôt qu'on extrait.** La
fiche d'équipe empruntait le style de sa table de staff à la feuille du widget
roster. Les règles ont été recopiées dans la feuille de la page, déclarations
inchangées — extraire un composant aurait demandé de choisir entre deux
définitions divergentes de `.player-table`, ce qui est un changement de rendu.
Une fois les portées posées, la duplication est inoffensive : c'est l'absence de
portée qui produit les collisions, pas la répétition.

## Le verrou

`scripts/check-css-collisions.sh`, deux contrôles :

- **A — portée** : toute règle d'une feuille de `pages/` ou `widgets/` commence
  par la classe qui porte le nom du fichier. C'est la règle durable ; elle rend
  les collisions entre feuilles de page **impossibles par construction**, pas
  seulement absentes.
- **B — collisions** : aucun sélecteur divergent entre feuilles, sur le même
  périmètre que A. Une collision avec une feuille hors application est inerte, et
  un verrou qui rougit sur du code mort finit ignoré.

Il **n'est pas encore branché** sur `make lint` ni sur la CI : il le sera à la fin
de l'épic E03, où son passage vaudra recette du périmètre. D'ici là il sert de
compteur, et il sort en erreur dès aujourd'hui plutôt que de tolérer un état
transitoire qu'on oublierait de retirer.

## Résultat

| | Avant | Après |
|---|---|---|
| Feuilles conformes | 1 / 49 | **46 / 46** |
| Sélecteurs divergents | 116 | **2** |
| Propriétés rendues modifiées | — | 2, assumées et documentées |

Les deux collisions restantes font l'objet de la **carte 359** : `.ts-team-name`
et `.ts-team-meta` portent deux tailles de police selon la feuille. Le composant
ne peut pas être scopé — il déborde de sa racine — et choisir un gagnant est un
changement de rendu, que cette carte renvoie explicitement ailleurs.

## Hors périmètre

**Toute harmonisation visuelle.** On ne modifie que la portée des règles, jamais
leur contenu. Un `letter-spacing` à 0,4 px d'un côté et 0,5 px de l'autre reste
tel quel.

**Les douze feuilles mortes.** Six ne servent qu'aux maquettes, six ne servent à
rien. Les supprimer est une carte de nettoyage séparée, déjà prévue par l'épic.

**Deux défauts de template rencontrés et laissés** : `schedule.html` porte deux
attributs `class` sur le même élément — le parseur garde le premier, donc
`.schedule-actions-panel` ne s'applique jamais ; `groups.html` porte un `style`
inline, que `CLAUDE.md` interdit. Corriger l'un ou l'autre changerait le rendu.

## Checklist

- [x] L'inventaire a été **remesuré** avant de commencer — 116 divergents, pas 79
- [x] Le harnais est écrit et vérifié : déterministe, et il attrape un changement
      délibéré en le nommant
- [x] Les références sont prises avant la première modification de CSS
- [x] Lot 1 — plus aucun `:root` dans `pages/` ni `widgets/`
- [x] Lot 2 — les 46 feuilles du périmètre portent leur portée, en sept grappes
      calculées, chacune vérifiée à zéro écart avant d'être commitée
- [x] Lot 3 — six des huit dernières collisions soldées ; les deux autres
      renvoyées à la carte 359 avec leur motif
- [x] Les deux valeurs qui changent sont mesurées, nommées et documentées
- [x] `scripts/check-css-collisions.sh` existe, sort en erreur, et compte
- [x] `make check-arch` passe
- [ ] Le verrou est branché sur `make lint` et la CI — **après la carte 359**,
      à la fin de l'épic E03
