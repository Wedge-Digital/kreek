# Retrait des trois réglages morts de l'étape 3

**Spec :** `docs/specs/notifications/configuration/07-integration.md`, et R10
**Dépend de :** 333
**Frotte avec :** la spec de retrait des play-offs — même template

> **Ne pas livrer seule.** Cette carte fait partie d'une chaîne de dix. Aucune
> carte avant la 340 ne fait partir un email : livrer `configuration/` sans
> `envoi/` produirait un **troisième interrupteur email mort**, mieux dessiné
> que les deux qu'il remplace et tout aussi inerte — le défaut même que cette
> fonctionnalité corrige. Cf. `docs/specs/notifications/README.md`.

## Objectif

Retirer de l'étape 3 du magicien trois réglages **stockés et lus par personne**.

| Réglage | Ce qu'il promettait | Remplacé par |
|---|---|---|
| `use_mail_notification` | « des rappels à l'ouverture et à la fermeture de la phase » | les notifications 2 et 3 |
| `schedule_timezone` | un fuseau par compétition | rien — R10 retient celui du serveur |

`notify_by_email` (étape 4) est déjà parti avec la carte 333.

## Conception

Retirer un champ d'une struct serde **ne casse pas** la lecture des blobs
existants : les clés inconnues sont ignorées. Aucune réécriture des ~399 blobs
`structure` n'est nécessaire, et mieux vaut s'en abstenir — gain cosmétique,
risque réel.

`assets/league_structure.json` porte `"use_mail_notification": true` et doit être
nettoyé : un fichier de référence citant un réglage disparu induit en erreur son
prochain lecteur.

Le VO `Timezone` (`shared_kernel/bloodbowl/timezone.rs`) n'aura **plus aucun
utilisateur**. Le supprimer aussi, plutôt que de laisser un type orphelin.

## Ordonnancement

`new-competition-phase-3.html` est aussi le terrain de la spec de retrait des
play-offs. Celle des deux qui passe en second reprend le fichier de l'autre — à
vérifier avant de commencer.

## Checklist

- [x] Bloc « Notifications e-mail » retiré du template, avec `setMailNotif` et
      les trois références à `state.useMailNotification`
- [x] Sélecteur de fuseau retiré, avec `state.scheduleTimezone`
- [x] `use_mail_notification` et `schedule_timezone` retirés de `ScheduleConfig`
- [x] `Timezone` supprimé s'il n'a plus d'utilisateur — le vérifier, ne pas le
      supposer
- [x] `assets/league_structure.json` nettoyé
- [x] Non-régression : l'étape 3 enregistre toujours, et une structure existante
      se relit sans erreur
- [x] `make check-arch` et `make e2e`

## Ce qui a été fait

Quatorze points d'usage, tous recensés avant de commencer : onze dans
`new-competition-phase-3.html`, deux champs et un type dans
`competition_structure.rs`, deux clés dans `assets/league_structure.json`. Plus
une trace laissée par la carte 331 — la fixture de test de
`competition_notifications.rs` citait `use_mail_notification`. Serde l'aurait
ignorée, mais un fixture qui nomme un champ disparu induit en erreur son
prochain lecteur.

`Timezone` n'avait bien qu'un seul utilisateur, vérifié et non supposé. Le
fichier et sa déclaration de module partent avec.

**Aucun conflit d'ordonnancement** : la carte prévenait que
`new-competition-phase-3.html` est aussi le terrain de la spec de retrait des
play-offs, mais celle-ci n'a aucune carte en `ready_to_be_done/` ni en
`to_be_refined/`. Rien n'était en vol sur ce fichier.

## Deux choses que le retrait a failli emporter

**Un `#[serde(default)]` orphelin.** L'attribut appartenait à
`schedule_timezone` ; les deux champs retirés, il s'est retrouvé collé à
`scheduled_dates`, qui était **obligatoire**. Une structure sans journées se
serait alors désérialisée au lieu d'échouer. Repéré à la relecture du diff, pas
par le compilateur — c'est le genre de changement qu'aucun test n'aurait signalé.

**Un test disparu**, 1049 → 1048. Ce n'est pas une perte de couverture :
`nutype` **engendre** un test par type à défaut validé
(`should_have_valid_default_value`), et celui de `Timezone` est parti avec lui.
Vérifié dans la liste des tests plutôt que supposé.

## La non-régression, vérifiée sur de la vraie donnée

**205 saisons** portent encore les deux clés dans leur blob `structure`. Sur la
plus ancienne, les trois pages qui lisent la structure répondent **200** — la
synthèse d'administration, le détail de compétition et l'étape 3 du magicien —
et la carte Structure de la synthèse est bien rendue.

C'est le point que la carte demandait de vérifier sans le déduire : qu'une
structure **écrite avant le retrait** se relise. Le parcours de création, lui,
n'exerce que des blobs neufs.

Suite e2e complète : 199 passés, 7 ignorés.
