# E10 — Référentiels éditables

**État :** 2 cartes · 0 faite

## La fonction

Le BC `references` sert les données statiques du jeu — rosters, positions,
compétences, coups de pouce — et les autres BCs les consomment par port et
adapter. Ces données sont aujourd'hui **figées dans le code ou la
configuration** : les faire évoluer demande un déploiement.

L'épic ouvre deux brèches dans ce référentiel : un référentiel de **ligues**
avec son sélecteur, et un **éditeur de rosters** permettant d'ajuster les
listes sans passer par le code.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 54 | Référentiel des ligues + widget sélecteur | fragment HTMX à 3 états, `on_select` injecté par l'hôte |
| **439 à 447** | **Roster personnalisé, propre à un espace** | neuf cartes issues du workflow feature, spécifiées dans `docs/specs/roster-personnalise/` |
| 50-reference-BC-roster-editor | Éditeur de rosters | **absorbée** par les neuf ci-dessus — sa question « modifier les rosters dans une certaine mesure » est tranchée : on n'édite pas ceux du règlement, on en crée de nouveaux |

## Ce qui commande l'ordre

Aucune dépendance entre les deux, mais **54 est de loin la plus avancée** : sa
conception est écrite — route, paramètres, les trois états du fragment,
l'intégration côté `team_creation` — et il ne lui reste que trois questions
ouvertes, toutes tranchables en une conversation :

- les ligues sont-elles définies en dur ou dans un fichier de config ? Et si
  config, quel rechargement ?
- la réponse du callback `on_select` re-sert-elle le fragment `references`, ou
  renvoie-t-elle un `HX-Trigger` ?
- faut-il une option « Sans affiliation » ?

`50-reference-BC-roster-editor` n'a qu'une phrase d'intention : *« permettre de
modifier, dans une certaine mesure, les rosters »*. C'est le « dans une
certaine mesure » qui est tout le sujet — un roster déjà utilisé par des équipes
existantes ne peut pas changer librement sans invalider des valeurs d'équipe
calculées. La question à trancher en premier : **l'édition porte-t-elle sur les
rosters d'une saison à venir seulement, ou sur ceux déjà en jeu ?**

Le patron d'intégration est acquis des deux côtés : le BC `references` expose un
fragment, l'hôte le compose, et le BC ne connaît aucune route de son
consommateur — c'est déjà ce que fait le widget de sélection de roster de la
page de construction d'équipe.

## Ce que l'épic ne couvre pas

- **Les autres référentiels** — compétences, coups de pouce, star players —
  qui restent en lecture seule.
- **Un back-office général.** Ces deux cartes ouvrent deux points d'édition
  précis, pas une administration du référentiel.

## Terminé quand

Un organisateur choisit la ligue de sa compétition depuis l'application, et
ajuste un roster sans qu'un déploiement soit nécessaire.
