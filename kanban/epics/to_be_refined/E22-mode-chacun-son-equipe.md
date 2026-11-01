# E22 — Le mode « Chacun son équipe »

**État :** `to_be_refined` — 10 cartes à écrire. Découpage validé le 2026-11-01. Née le 2026-11-01 des
maquettes mobiles. Dépend de E19 (camp du coach courant), E20 (annulation des
coups de pouce) et E21 (mode de saisie). **Se livre après E23** (ordre décidé
le 2026-11-01 : E21, E23, puis E22) : ses attentes profitent du direct.

**Maquettes :** `app-mobile-parcours.html`, sections 3 et 3B.

## La fonction

Deux coachs saisissent le même match en même temps, chacun sur son téléphone.
**Tout le parcours est coupé en deux** (décision du 2026-11-01) : chaque coach
fait son avant-match — son fan factor, ses coups de pouce —, ses actions, puis
son après-match — son MVP, ses gains, son fan factor, son paragraphe de résumé —
et valide son camp. Ce qui est à l'autre s'affiche en lecture seule.

Aujourd'hui, l'avant-match et l'étape 5 saisissent les deux camps dans un seul
formulaire et un seul événement (`FanFactorRecorded`, `PostMatchRecorded`) : en
parallèle, le dernier qui enregistre écrase l'autre sans le savoir.

Mêmes routes qu'en « Je saisis tout » : leur comportement change avec le mode
(principe posé par E21).

Règles :

- **Coups de pouce** : la plus forte valeur d'équipe achète d'abord. Le favori
  déclare « J'ai fini mes achats », même sans rien acheter ; l'outsider attend
  jusque-là. Une erreur se corrige par l'annulation globale de E20.
- **Résumé** : chacun écrit son paragraphe ; les deux sont fusionnés dans le
  récapitulatif, chacun le sien. **Le titre est à la main du coach à domicile**
  (décision du 2026-11-01).
- **Validation** : chacun valide son camp ; la seconde validation publie.
- **Correction après publication** : l'un ou l'autre des coachs peut dépublier,
  selon les règles actuelles — tant qu'aucun SPP n'a été dépensé
  (`correction_eligibility_service`) ; la nouvelle publication demande de
  nouveau les deux validations (décisions du 2026-11-01).
- **Démarrage concurrent** : deux coachs cliquent « Commencer » en même temps —
  le premier choix de mode validé l'emporte ; l'autre arrive sur l'état du match.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| — | Le second coach rejoint le match | le mode est déjà choisi : il voit où en est l'adversaire et part à son avant-match ; le premier « Commencer » validé l'emporte |
| — | L'avant-match de son camp | un événement de fan factor par camp ; l'étape 2 réduite à son camp, l'autre en lecture seule |
| — | Le favori a fini ses achats | « J'ai fini mes achats », même vide ; l'outsider attend jusque-là, la route refuse ses achats avant |
| — | Les actions de son camp | accès à la seule étape de son camp ; « ＋ Action de mon équipe » sans choix d'équipe ; ✕ sur ses seules actions |
| — | Le MVP de son camp | l'étape MVP réduite à son camp, l'état de l'adversaire affiché |
| — | Les gains et le fan factor de son camp | un événement d'après-match par camp ; l'étape 5 réduite à son camp |
| — | Deux paragraphes de résumé | chacun le sien, fusionnés dans le récapitulatif ; le titre au coach à domicile |
| — | Valider son camp | « publier » devient « valider mon camp » ; la seconde validation publie ; le récapitulatif affiche « validé par » pour chaque camp, à côté du « publié par » de E19 |
| — | Corriger un compte rendu à deux | l'un ou l'autre coach dépublie, aux règles actuelles ; les deux validations sont remises à zéro et redemandées |
| — | Les tests e2e à deux coachs | deux contextes de navigateur, un par coach, sur le même match |

## Ce qui commande l'ordre

Le second coach qui rejoint vient d'abord : sans lui, le mode B n'a qu'un
coach. Puis le parcours dans l'ordre du match — avant-match, coups de pouce,
actions, MVP, gains, résumé, validation —, chaque étape livrable seule tant que
les suivantes restent en « tout dans un formulaire ».

Les événements par camp (fan factor, après-match) doivent laisser intacts les
événements à deux camps de « Je saisis tout » : l'historique se rejoue tel
quel.

## Conséquences sur les données de production

**Aucun compte rendu existant n'est en mode B** : le mode n'existe que pour les
matchs démarrés après E21. Le risque porte sur la cohabitation des formes :

| Carte | Données modifiées | Effet sur l'existant |
|---|---|---|
| Rejoindre | aucune | — |
| Avant-match par camp | un événement **ajouté** | `FanFactorRecorded`, à deux camps, reste celui du mode A ; l'agrégat rejoue les deux formes. Les valeurs d'équipe se figent au premier fan factor enregistré, pour les deux équipes |
| Favori a fini | un événement **ajouté** | propre au mode B ; le mode A enchaîne comme aujourd'hui |
| Actions, MVP | aucune | les actions sont déjà stockées par camp |
| Après-match par camp | un événement **ajouté** | `PostMatchRecorded` reste celui du mode A. **Le `MatchReportPublished` émis vers `teams`, `ranking`, `players` et `competitions` ne change pas** : les deux camps y sont assemblés |
| Deux paragraphes | des champs **ajoutés**, mode B seul | le résumé ne sort pas du BC (vérifié) : aucun autre contexte touché |
| Valider son camp | un événement **ajouté** | la publication émet le même événement qu'aujourd'hui ; `published_by` est le second validateur |
| Correction à deux | aucune | règle d'éligibilité actuelle, inchangée |

## Ce que l'épic ne couvre pas

- **Le direct** : E23, livrée avant cette épic ; les attentes de l'outsider et
  des validations s'y branchent.
- **Changer de mode en cours de match** : exclu.

## Terminé quand

Deux coachs, chacun dans son navigateur, saisissent le même match : chacun son
fan factor, l'outsider achète après que le favori a fini, chacun ses actions, son
MVP, ses gains et son paragraphe. Aucun ne peut écrire dans le camp de l'autre —
une tentative directe sur la route reçoit un refus. Le compte rendu se publie à
la seconde validation, avec les deux paragraphes du résumé.
