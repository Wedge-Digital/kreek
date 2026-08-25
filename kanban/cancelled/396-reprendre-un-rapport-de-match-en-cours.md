> **Carte annulée le 2026-08-25 — incompatible avec la correction d'un rapport.**
>
> Le mécanisme décrit ici — une progression dérivée qui pilote la reprise, et un
> effacement des actions au recul vers les coups de pouce — suppose qu'un
> rapport se remplit une fois, dans un sens. Or la **correction** repose sur
> l'inverse : les quatre étapes acceptent explicitement l'état
> `ReadyToPublish` (`pre_match_controller.rs:89`,
> `inducements_controller.rs:163`, `actions_step_controller.rs:79`,
> `step5_controller.rs:88`), et c'est ainsi qu'on rouvre n'importe quelle étape
> d'un rapport complet, y compris après dépublication.
>
> Trois collisions, dont une destructrice :
>
> 1. la progression dérivée ne distingue pas « en cours de saisie » de
>    « complet, en correction » : un rapport dépublié a tous ses jalons remplis
>    et serait renvoyé au récapitulatif, jamais là où le correcteur veut aller ;
> 2. **le rinçage effacerait les actions d'un match déjà joué** dès qu'un
>    correcteur recule vers les coups de pouce — un risque que le code
>    n'a pas aujourd'hui, et que cette carte introduirait ;
> 3. le même bouton « Retour » sert les deux modes, et rien dans l'agrégat ne
>    les distingue : `ReadyToPublish` signifie à la fois « saisie finie » et
>    « dépublié pour correction ».
>
> **Ce qui reste vrai et devra être repris ailleurs** — le constat, lui, n'est
> pas annulé :
>
> - rouvrir un rapport entamé ramène **toujours** à l'étape 2, quel que soit
>   l'avancement, `PreMatch` couvrant quatre étapes ;
> - le « ← Retour » de l'étape 2 pointe vers `edit_match_report`, qui redirige un
>   rapport en `PreMatch` vers l'étape 2 : **le bouton boucle sur lui-même** ;
> - le facteur fans se réenregistre sans aucune garde, y compris depuis
>   `ReadyToPublish` que `load_pre_match` ramène en arrière par
>   `rtp.into_pre_match()`, ce qui réémet `TeamValuesRecorded` et déplace
>   l'écart de valeur d'équipe sur lequel les coups de pouce ont été achetés.
>
> Une carte de reprise qui tiendrait compte de la correction reste possible ;
> elle partirait d'une décision de modèle que celle-ci n'a pas prise —
> distinguer première saisie et correction, par exemple sur la présence d'un
> `MatchReportPublished` dans le flux.

# Reprendre un rapport de match en cours, sans refaire ce qui est fait

**Priorité : haute**
**Dépend de :** rien
**Fichiers :** `src/app/match_report/domain/match_report_pre_match.rs`,
`match_report_state.rs`, `events.rs`,
`src/app/match_report/io/web/match_selection_controller.rs`,
`actions_step_controller.rs`, `pre_match_controller.rs`,
`inducements_controller.rs`, `src/app/match_report/routes.rs`

## Objectif

Deux comportements, un seul mécanisme dessous.

1. **Rouvrir un rapport entamé mène à l'étape en cours**, pas au début.
2. **Reculer des actions vers les coups de pouce demande une confirmation** et
   efface les actions saisies — et seulement dans ce cas.

## L'état de progression : dérivé, jamais persisté

La progression se **calcule depuis les données remplies**. Pas d'état
supplémentaire en base : l'agrégat est rejoué depuis l'event store, un état
persisté pourrait diverger de ce qu'il décrit, et il faudrait le migrer.

Les jalons existent déjà, et ils distinguent « pas encore fait » de « fait, et
vide » — la difficulté habituelle de ce genre d'état :

| Jalon | Champ | « Pas fait » |
|---|---|---|
| Sélection confirmée | l'état `PreMatch` lui-même | état `Draft` |
| Facteur fans + TV | `home_fan_roll` / `away_fan_roll` | `None` |
| Coups de pouce domicile | `home_inducements` | `None` — `Some([])` = aucun achat, **et c'est fait** |
| Coups de pouce extérieur | `away_inducements` | `None` |
| Actions domicile | `home_actions` | **rien ne le dit** — voir ci-dessous |
| Actions extérieur | `away_actions` | idem |
| Après-match | l'état `ReadyToPublish` | — |

Un achat vide émet bien un `InducementsRecorded` : un test du domaine le
verrouille (`match_report_pre_match.rs`, « home_inducements est Some et vide »).

**Le seul trou : les actions.** `home_actions` et `away_actions` sont des `Vec`
nus. Une équipe qui n'a rien fait du match — aucun touchdown, aucune passe,
aucune blessure — est indiscernable d'une équipe dont la saisie n'a pas
commencé. Il faut un jalon explicite. Deux formes possibles, à choisir à
l'implémentation :

- passer les deux champs en `Option<Vec<MatchAction>>`, comme les coups de pouce ;
- ou émettre un événement au franchissement de l'étape.

La première est cohérente avec ce qui existe déjà pour les coups de pouce.

## La reprise à l'ouverture

`edit_match_report` (`match_selection_controller.rs:160`) ne regarde
aujourd'hui que l'état de l'agrégat, et `PreMatch` couvre **quatre étapes** :
un coach interrompu en pleine saisie des actions rouvre son rapport sur le
formulaire du facteur fans.

| Progression | Destination |
|---|---|
| Sélection non confirmée | formulaire de sélection |
| Facteur fans absent | étape 2 |
| Coups de pouce domicile absents | coups de pouce, équipe domicile |
| Coups de pouce extérieur absents | coups de pouce, équipe extérieure |
| Actions domicile non commencées | actions, équipe domicile |
| Actions extérieur non commencées | actions, équipe extérieure |
| Tout saisi | après-match, puis récapitulatif |

## Le rinçage : un seul cas

**Uniquement** le passage « actions de l'équipe domicile → coups de pouce ».
C'est le seul recul qui traverse la frontière entre deux natures de saisie.

- **Navigation libre entre les deux camps.** Passer des actions d'une équipe à
  celles de l'autre n'efface rien, dans les deux sens.
- **Confirmation avant d'effacer.** Un avertissement disant que les actions
  saisies seront perdues, et une validation explicite. Inutile de compter ni de
  détailler ce qui sera effacé.
- **Aucun avertissement si rien n'a été saisi.** Le recul est alors une
  navigation ordinaire.
- **Si l'utilisateur valide, toutes les actions sont effacées** — les deux
  camps : on remonte avant l'étape de saisie, pas dans l'un de ses deux volets.

## Deux conséquences techniques

**Le « ← Retour » cesse d'être un lien.** C'est aujourd'hui un `<a href>`, donc
un GET. Un GET ne doit pas détruire de données : un préchargement de navigateur
ou un lien recopié suffirait à effacer un match. Ce bouton devient un POST,
soumis au middleware CSRF qui exige `HX-Request: true`.

**« Effacer » n'efface rien.** Dans un event store, c'est une compensation.
`ActionDeleted` existe, mais **unitaire** : quarante actions donneraient quarante
événements et autant de lignes de journal. Il faut un événement de lot, nommé en
fait domaine — ce qui s'est passé, pas l'écran d'où vient le clic.

## Trois défauts constatés au passage

1. **L'ouverture ramène toujours à l'étape 2** — l'objet même de cette carte.
2. **Le « ← Retour » de l'étape 2 boucle sur elle-même** : il pointe vers
   `edit_match_report`, qui redirige un rapport en `PreMatch`… vers l'étape 2.
   Bouton inopérant, à recâbler vers la sélection ou à retirer.
3. **Le facteur fans se réenregistre sans garde**, y compris depuis l'état
   `ReadyToPublish` que `load_pre_match` ramène en arrière par
   `rtp.into_pre_match()`. Hors périmètre ici, mais à connaître : il réémet
   `TeamValuesRecorded`, donc l'écart de TV.

## Ce que la carte ne couvre pas

- **Le budget de coups de pouce après un facteur fans rejoué.**
  `record_inducements_use_case` ne recalcule les TV que si elles sont absentes :
  les achats déjà faits restent adossés à un écart de TV qui a changé.
- **L'après-match périmé par une correction d'actions.** Modifier des actions
  après avoir saisi les gains ne les recalcule pas.

Les deux sont des incohérences réelles, écartées **délibérément** de cette
carte : elles relèvent de la correction d'un rapport, pas de sa reprise.

## Checklist

- [ ] Jalon explicite pour les actions — `Option<Vec<MatchAction>>` ou événement
      de franchissement
- [ ] Progression dérivée, portée par le domaine et non par un contrôleur
- [ ] `edit_match_report` redirige selon la progression
- [ ] Le retour « actions domicile → coups de pouce » passe en POST, avec
      confirmation
- [ ] Événement de lot d'effacement des actions, appliqué au domaine et à la
      projection dans la même transaction
- [ ] Le « ← Retour » de l'étape 2 recâblé ou retiré
- [ ] Tests unitaires :
  - [ ] progression pour chaque combinaison de jalons, dont
        « coups de pouce faits sans achat » qui ne doit pas ramener en arrière
  - [ ] aller-retour entre les deux camps : aucune action perdue
  - [ ] recul vers les coups de pouce : les deux camps sont vidés
  - [ ] recul sans aucune action saisie : aucune confirmation, aucun événement
- [ ] Test e2e : saisir des actions, revenir aux coups de pouce, confirmer,
      vérifier que la saisie est vide ; puis rouvrir le rapport et vérifier
      qu'on arrive à l'étape des coups de pouce
