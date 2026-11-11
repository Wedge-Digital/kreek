# E22 — Le mode « Chacun son équipe »

**État :** `en_cours` — 10 cartes, 652 à 661 ; 2 faites (652, 653). Découpage validé le 2026-11-01 ;
cadrage et décisions le 2026-11-11.
**Cadrage :** `docs/specs/saisie-mobile/E22-cadrage.md`. Née le 2026-11-01 des
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
formulaire et un seul événement (`FanFactorRecorded`, `PostMatchRecorded`) :
quand les deux coachs enregistrent l'un après l'autre, le second écrase l'autre
sans le savoir. En simultané, l'event store refuse la seconde écriture (409),
et la page qui la portait se recharge en perdant sa saisie — d'où la carte 653.

Mêmes routes qu'en « Je saisis tout » : leur comportement change avec le mode
(principe posé par E21).

Règles :

- **Coups de pouce** : la plus forte valeur d'équipe achète d'abord. Le favori
  déclare « J'ai fini mes achats », même sans rien acheter ; l'outsider attend
  jusque-là. Une erreur se corrige par l'annulation globale de E20. **L'ordre et
  le refus existent déjà** (carte 627) : `InducementsRecorded`, même vide, dit
  que le favori a fini — aucun événement n'est à créer.
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
  **Déjà tenu par E21** (409 et rechargement, ou ouverture silencieuse) : aucune
  carte n'y ajoute rien.

Décisions du 2026-11-11 (cadrage, § 12) : l'option « Chacun son équipe »
s'ouvre à la 659 ; une commande d'un camp est réessayée une fois sur conflit ;
le match commence quand les deux avant-matchs et les deux sélections de coups
de pouce sont enregistrés ; la validation est un geste distinct, possible dès
que son camp est prêt ; un camp validé est figé, sa validation se retire tant
que l'autre n'a pas validé ; l'admin écrit et valide les deux camps ; seule la
validation fait un toast ; `PostMatchClosed` dit le passage en
`ReadyToPublish` ; un coach des deux équipes ne se voit pas proposer le mode B.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 652 | Le second coach rejoint le match | le mode `TeamEntry` ; le droit d'écrire par camp et sa garde ; le second coach part à son avant-match |
| 653 | Deux coachs écrivent en même temps | **ajoutée le 2026-11-11** : une commande d'un camp est réessayée une fois sur conflit de version, au lieu d'un rechargement qui perd la saisie |
| 654 | L'avant-match de son camp | un événement de fan factor par camp, émis ; valeurs d'équipe figées au premier ; l'étape 2 réduite à son camp, l'autre en lecture seule et en direct |
| 655 | Le favori a fini ses achats | « J'ai fini mes achats », même vide, **sans événement nouveau** ; l'écran d'attente de l'outsider, qui s'ouvre au direct ; chacun n'achète que pour son équipe |
| 656 | Les actions de son camp | accès à la seule étape de son camp ; « ＋ Action de mon équipe » sans choix d'équipe ; ✕ sur ses seules actions ; le coup d'envoi après les deux avant-matchs |
| 657 | Le MVP de son camp | l'étape MVP réduite à son camp, sans attendre l'autre, l'état de l'adversaire affiché |
| 658 | Les gains, les fans et le paragraphe de son camp | **fusion** des cartes « gains et fan factor » et « deux paragraphes » : un événement d'après-match par camp, émis ; l'étape 5 réduite à son camp ; le titre au domicile ; les deux paragraphes au récapitulatif |
| 659 | Valider son camp | « publier » devient « valider mon camp » ; la seconde validation publie ; « validé par » pour chaque camp, à côté du « publié par » de E19 ; l'option « Chacun son équipe » s'ouvre |
| 660 | Corriger un compte rendu à deux | l'un ou l'autre coach dépublie, aux règles actuelles ; les deux validations sont remises à zéro et redemandées |
| 661 | Les tests e2e à deux coachs | deux contextes de navigateur, un par coach, sur le même match |

## Ce qui commande l'ordre

Le second coach qui rejoint vient d'abord : sans lui, le mode B n'a qu'un
coach, et sa carte pose le droit par camp dont toutes les autres dépendent. La
653 suit : dès que deux coachs écrivent, leurs écritures se croisent. Puis le
parcours dans l'ordre du match — avant-match, coups de pouce, actions, MVP,
gains et résumé, validation —, chaque étape livrable seule tant que les
suivantes restent en « tout dans un formulaire ». Le mode B ne se choisit dans
l'interface qu'à la 659 : avant, il ne se démarre que par un POST de test.

Les événements par camp (fan factor, après-match) doivent laisser intacts les
événements à deux camps de « Je saisis tout » : l'historique se rejoue tel
quel.

## Conséquences sur les données de production

**Aucun compte rendu existant n'est en mode B** (vérifié : `EntryMode` n'a
qu'une variante, et toute la base de dev est en `SoloEntry`). Le risque porte
sur la cohabitation des formes :

| Carte | Données modifiées | Effet sur l'existant |
|---|---|---|
| 652 Rejoindre | une **valeur** ajoutée, `TeamEntry`, dans `EntryModeChosen` et `match_report_proj.entry_mode`, sans migration | aucun ; trois tests qui prenaient `TeamEntry` pour un mode inconnu changent de valeur |
| 653 Écritures simultanées | aucune | en mode A aussi, un conflit sur une commande de camp est réessayé une fois |
| 654 Avant-match par camp | un événement **ajouté** | `FanFactorRecorded`, à deux camps, reste celui du mode A ; l'agrégat rejoue les deux formes. **Le gel des valeurs d'équipe au premier fan factor est une règle nouvelle du mode B** : en mode A, chaque fan factor les relit, comme aujourd'hui |
| 655 Favori a fini | aucune | aucun événement : `InducementsRecorded` porte déjà la validation du favori ; le mode A enchaîne comme aujourd'hui |
| 656, 657 Actions, MVP | aucune | les actions sont déjà stockées par camp, avec leur auteur |
| 658 Après-match et paragraphes | deux événements **ajoutés**, des champs de paragraphe dans l'agrégat, mode B seul | `PostMatchRecorded` reste celui du mode A. **Le `MatchReportPublished` émis vers `teams`, `ranking` et `competitions` ne change pas** — `players` reçoit, lui, les `PlayerMatchImpactAppEvent` du publisher, inchangés aussi : les deux camps y sont assemblés. Le résumé ne sort pas du BC (vérifié). Le récapitulatif affiche désormais un corps de résumé sans titre, qu'il masquait |
| 659 Valider son camp | deux événements **ajoutés** (validation, retrait) | la publication émet le même événement qu'aujourd'hui ; `published_by` est le second validateur |
| 660 Correction à deux | aucune | règle d'éligibilité actuelle, inchangée |
| 661 Tests | aucune | — |

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
