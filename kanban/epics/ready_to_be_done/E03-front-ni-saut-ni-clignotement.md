# E03 — Front : ni saut, ni clignotement

**État :** 5 cartes · 2 faites — 341 et 342 livrées et vérifiées.
Le **clignotement est supprimé** ; restent le saut (343) et deux cartes de dette (17, 18).

## La fonction

L'application bouge sous les yeux de l'utilisateur pendant les premières
centaines de millisecondes de chaque page. Deux phénomènes distincts s'y
superposent :

- **le clignotement** — le contenu est peint sans ses styles pendant 50 à
  200 ms, parce que chaque fragment transporte son propre `<link>` et qu'un
  `<link>` inséré dans un DOM vivant ne bloque pas le rendu ;
- **le saut** — jusqu'à **280 px de déplacement vertical cumulé** sur
  l'accueil, parce que les zones remplies en différé ont une hauteur nulle
  jusqu'à l'arrivée de leur fragment.

L'épic supprime les deux. Elle emporte aussi les deux dernières dépendances CDN
et les scripts inline des fragments — même famille de dette, même fichier de
layout.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 341 | Donner sa portée à chaque feuille, à rendu constant | **faite** — 116 collisions ramenées à 2 ; rend la fusion possible |
| 342 | Les feuilles réunies en un fichier unique dans le `<head>` | **faite** — 0 `<link>` et 0 requête CSS au swap |
| 343 | Réserver la place des zones remplies en différé | **supprime le saut** |
| 17 | Dépendances CDN en production | TomSelect et le widget Cloudinary servis depuis `/static` |
| 18 | Scripts inline dans les fragments HTMX | init par `htmx:afterSwap`, plus de `<script>` par fragment |

## Ce qui commande l'ordre

**342 dépend de 341 terminée *et vérifiée*.** Fusionner avant d'avoir soldé les
collisions produit des régressions visuelles silencieuses : tant que les
feuilles sont isolées les divergences sont invisibles, et le bundle ne les
neutralise pas — il les active.

**341 et 342 sont faites, et le clignotement a disparu.** Mesuré sur une
navigation HTMX : **0 `<link>` inséré, 0 requête CSS**, contenu stylé à
l'arrivée. Le premier rendu, seule contrepartie redoutée, ne bouge pas — +12 ms
sur une page, −8 ms sur l'autre, dans le bruit.

**La 341 était nécessaire et pas suffisante**, ce que seule la fusion a montré :
elle a d'abord modifié 13 853 valeurs calculées sur 32 pages. L'isolation des
feuilles **globales** ne venait pas de leurs sélecteurs mais du fait qu'on ne
les chargeait pas — et c'est exactement ce que le bundle supprime. Il a fallu un
troisième contrôle, `tests/e2e/visual/debordements.py`, pour poser la question
que le verrou ne savait pas poser : *ce sélecteur trouve-t-il du markup sur une
page qui ne chargeait pas sa feuille ?*

**343 est indépendante** des deux autres et peut se faire à tout moment, avant
comme après. Elle survivrait à 341 et 342 : le saut n'est pas un problème de
CSS mais de hauteur non réservée.

**17 et 18** sont indépendantes de tout le reste.

## Ce que l'épic ne couvre pas

- **Toute harmonisation visuelle.** 341 et 342 posent la même contrainte : on
  ne modifie que la *portée* des règles, jamais leur *contenu*. Un
  `letter-spacing` à 0,4 px d'un côté et 0,5 px de l'autre reste tel quel. Si
  on le veut, c'est une autre carte, qui s'assume comme un changement visuel.
- **Les 13 feuilles mortes** référencées seulement par les maquettes
  `assets/templates/` — 342 les exclut du bundle, les supprimer du dépôt est
  une carte de nettoyage séparée.
- **Le rendu serveur du menu.** 343 supprime l'*effet visuel* de la requête de
  menu, pas la requête. La rendre côté serveur dans le layout est plus lourd —
  il faut que chaque page dispose du contexte du menu — et fait une carte à
  part.
- **L'appel dupliqué à `/app/spaces`** — la sidebar figure deux fois dans
  `app-layout.html`, desktop et drawer mobile, chacune en `hx-trigger="load"`.
  Réel et mesuré, mais c'est un problème de requêtes, pas de mise en page.

## Terminé quand

Sur la démo, une navigation HTMX n'insère **aucun** `<link>` et ne déclenche
**aucune** requête CSS ; et un chargement complet de l'accueil produit un
déplacement vertical cumulé de **0 px**, mesuré en desktop et sous 768 px.
