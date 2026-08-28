# E10 — Référentiels éditables

**État :** 3 chantiers · 0 fait

## La fonction

Le BC `references` sert les données statiques du jeu — rosters, positions,
compétences, coups de pouce — et les autres BCs les consomment par port et
adapter. Ces données sont aujourd'hui **figées dans le code ou la
configuration** : les faire évoluer demande un déploiement.

L'épic ouvre trois brèches dans ce référentiel : un référentiel de **ligues**
avec son sélecteur, un **éditeur de rosters** permettant d'ajuster les listes
sans passer par le code, et des **compétences personnalisées** qu'un espace
ajoute à celles du règlement.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 54 | Référentiel des ligues + widget sélecteur | fragment HTMX à 3 états, `on_select` injecté par l'hôte — **`to_be_refined`**, sa section « Points à préciser » porte trois questions |
| **439 à 447** | **Roster personnalisé, propre à un espace** | neuf cartes issues du workflow feature, spécifiées dans `docs/specs/roster-personnalise/` |
| **463 à 472** | **Compétences personnalisées, propres à un espace** | dix cartes issues du workflow feature, spécifiées dans `docs/specs/competences-personnalisees/` |
| 50-reference-BC-roster-editor | Éditeur de rosters | **absorbée** par les neuf ci-dessus — sa question « modifier les rosters dans une certaine mesure » est tranchée : on n'édite pas ceux du règlement, on en crée de nouveaux |

## Ce qui commande l'ordre

**54 est indépendante des deux autres, et de loin la plus avancée** : sa
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

## Rosters et compétences partent ensemble

Ce n'est pas une commodité de planning : les deux se tiennent par les deux bouts.

Une compétence personnalisée peut être posée en **compétence de base d'un poste**
de roster personnalisé — donc le sélecteur de l'éditeur de roster (carte 446)
doit montrer celles de l'espace, et le compte d'usage d'une compétence
(carte 466) doit interroger la table des rosters d'espace (carte 441).

Livrer l'un sans l'autre donnerait deux fonctionnalités qui s'ignorent : un
espace pourrait créer une compétence et un roster, **sans pouvoir poser l'une
dans l'autre**, qui est pourtant leur emploi le plus évident ensemble.

Une seule contrainte de séquence en découle : la 466 vient après la 441.

## Ce que l'épic ne couvre pas

- **Les coups de pouce et les joueurs vedettes**, qui restent en lecture seule.
- **La duplication d'une entrée du règlement** pour la retoucher — ni pour un
  roster, ni pour une compétence. Ce geste mérite sa propre décision.
- **Un back-office général.** Ces trois chantiers ouvrent trois points d'édition
  précis, pas une administration du référentiel.

## Terminé quand

Un organisateur choisit la ligue de sa compétition depuis l'application, ajuste
un roster sans qu'un déploiement soit nécessaire, et **fait apprendre à l'un de
ses joueurs une compétence qu'il a écrite lui-même**.
