# Reprendre un rapport de match en cours, sans refaire ce qui est fait

**Priorité : haute**
**Dépend de :** rien
**Fichiers pressentis :** `src/app/match_report/io/web/match_selection_controller.rs`,
`pre_match_controller.rs`, `inducements_controller.rs`, `step5_controller.rs`,
`src/app/match_report/domain/match_report_pre_match.rs`,
`src/app/match_report/use_cases/record_fan_factor_use_case.rs`

## Le problème

Rouvrir un rapport de match déjà entamé renvoie le coach **au début**, et rien
ne l'empêche de re-valider une étape déjà validée.

## Ce que fait le code aujourd'hui — constat du 2026-08-25

**L'ouverture ramène toujours au même endroit.** `edit_match_report`
(`match_selection_controller.rs:160`) redirige selon le seul état de l'agrégat :

| État | Destination |
|---|---|
| `Draft` | formulaire de sélection des équipes |
| `PreMatch` | **étape 2 — facteur fans**, toujours |
| `ReadyToPublish` | étape 5 |
| `Published` | 409 |
| `Cancelled` | 410 |

`PreMatch` couvre **tout le match** : facteur fans, coups de pouce, première
mi-temps, seconde mi-temps. Un coach interrompu au milieu de la seconde
mi-temps rouvre son rapport… sur le formulaire du facteur fans.

**L'étape 2 se laisse re-soumettre.** Le contrôleur calcule bien
`fan_factor_already_recorded`, mais le template n'en fait qu'un message :
« Fan factor déjà enregistré — vous pouvez le modifier ci-dessous »
(`pre-match.html:99`). Le formulaire reste actif, et
`record_fan_factor_use_case` réémet `FanFactorRecorded` **et**
`TeamValuesRecorded` sans aucune garde.

**Depuis `ReadyToPublish` aussi.** `load_pre_match` accepte cet état et le
ramène en arrière par `rtp.into_pre_match()` : le facteur fans d'un rapport
prêt à publier peut être réenregistré.

Ce n'est pas neutre : le facteur fans détermine la recette, et la TV réenregistrée
change la petite monnaie de l'outsider — donc les coups de pouce déjà achetés.

## La question à trancher — ce qui rend cette carte « à raffiner »

Rejouer une étape est-il une **erreur à interdire** ou une **correction
légitime** ?

Les deux se défendent, et le code fait aujourd'hui les deux à moitié : il
autorise tout, et n'en dit rien. Trois voies :

1. **Interdire.** Une étape validée se ferme ; corriger passe par la
   dépublication, qui existe déjà (`unpublish_match_report_use_case`,
   `correction_eligibility_service`). Cohérent avec le reste du BC.
2. **Autoriser en le disant.** L'étape rouverte est en lecture, avec un bouton
   « Corriger » explicite, et un avertissement nommant ce que la correction
   déplace — recette, TV, coups de pouce.
3. **Autoriser tant que l'aval n'est pas saisi.** Le facteur fans se corrige
   tant qu'aucune action n'a été enregistrée, plus après. Le plus juste
   métier, le plus coûteux à tenir : il faut savoir, étape par étape, ce que la
   suivante a figé.

## Ce qu'il faudra de toute façon

Quelle que soit la voie, **l'agrégat doit savoir où en est la saisie** — ce
qu'il ne sait pas : `PreMatch` est un seul état pour quatre étapes. Aujourd'hui,
la progression ne se lit qu'en interrogeant des champs optionnels
(`home_fan_roll.is_some()`, présence d'actions, coups de pouce enregistrés), un
par contrôleur, sans que personne ne porte la question entière.

C'est la vraie carte : **une notion de progression dans le pré-match**, dont
découlent la reprise au bon endroit et le verrouillage de ce qui est fait.

## Ce que la carte ne couvre pas

- La correction d'un rapport **publié** — elle a son mécanisme
  (`unpublish`), ses gardes (SPP dépensés) et ses cartes.
- Le cloisonnement d'accès : qui a le droit d'ouvrir ce rapport est une autre
  question, déjà traitée ailleurs.

## À décider avant de passer en `ready_to_be_done`

- [ ] Laquelle des trois voies
- [ ] Si voie 3 : la table « ce que chaque étape fige pour les précédentes »
- [ ] Comment la progression est portée — champ dérivé de l'agrégat, ou état
      supplémentaire dans `MatchReportState`
- [ ] Ce que voit le coach en rouvrant : reprise directe à l'étape courante, ou
      sommaire des étapes avec leur état
