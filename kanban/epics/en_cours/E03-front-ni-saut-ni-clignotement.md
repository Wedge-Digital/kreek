# E03 — Front : ni saut, ni clignotement

**État :** 5 cartes · 4 faites — 341, 342, 343 et 17 livrées et vérifiées.
Le **clignotement est supprimé** et le **saut aussi**, sur le périmètre nommé.
Reste la carte 18. L'épic a engendré deux cartes hors périmètre : 361 (le saut
des dix autres pages) et 362 (le bundle gelé).

La 17 a déplacé le critère sans le contredire. Elle devait supprimer la dernière
requête CSS d'une navigation HTMX, celle du widget Cloudinary ; on a constaté en
la traitant que cette feuille ne stylait que l'iframe du widget, jamais notre
markup — donc qu'elle ne pouvait pas produire le clignotement que l'épic
combat. Le widget est conservé pour ses fonctions, son chargeur cantonné aux
deux pages qui l'utilisent. Le « Terminé quand » ci-dessous demande donc plus
que ce que l'épic voulait dire, et devra être reformulé avant clôture : « aucun
contenu peint sans ses styles » plutôt que « aucune requête CSS ».

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
| 343 | Réserver la place des zones remplies en différé | **faite** — 0 px sur les cinq pages du critère, desktop et mobile |
| 17 | Dépendances CDN en production | **faite** — 3 origines externes supprimées ; la 4e, Cloudinary, cantonnée aux 2 pages d'upload |
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

**343 était indépendante** des deux autres, et l'est restée : le saut n'est pas
un problème de CSS mais de hauteur non réservée.

**Elle a soldé deux zones, pas toutes.** La zone de menu se réserve avec un
token, sa hauteur étant connue d'avance. L'effectif est d'une autre nature : sa
hauteur dépend d'une donnée que la page ne peut pas connaître, et ce qui a rendu
la réservation sûre est qu'**onze joueurs est une règle du jeu** — aucune fiche
n'en affiche moins, donc jamais de blanc permanent. C'est le critère que la
carte 361 devra appliquer zone par zone : *existe-t-il un plancher aussi solide
qu'une règle du domaine ?* Sans lui, réserver au jugé remplace un saut de 100 ms
par un blanc permanent, ce qui est pire.

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
- **Le saut des dix pages hors critère** — 343 mesure zéro sur les cinq pages
  qu'elle nomme, et la mesure élargie en trouve dix autres, jusqu'à 1 841 px sur
  la construction d'équipe. Carte 361, qui reprend l'outil et la méthode.
- **Le bundle CSS gelé au démarrage** — effet de bord de 342 : une feuille
  éditée n'a aucun effet sur un serveur qui tourne, et rien ne le signale. C'est
  de l'ergonomie de développement, pas du rendu ; l'épic se constate sur la
  démo, où le défaut n'existe pas. Carte 362.
- **L'appel dupliqué à `/app/spaces`** — la sidebar figure deux fois dans
  `app-layout.html`, desktop et drawer mobile, chacune en `hx-trigger="load"`.
  Réel et mesuré, mais c'est un problème de requêtes, pas de mise en page.

## Terminé quand

Sur la démo, une navigation HTMX n'insère **aucun** `<link>` et ne déclenche
**aucune** requête CSS ; et un chargement complet de l'accueil produit un
déplacement vertical cumulé de **0 px**, mesuré en desktop et sous 768 px.
