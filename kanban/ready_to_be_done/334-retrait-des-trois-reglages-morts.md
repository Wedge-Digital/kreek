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

- [ ] Bloc « Notifications e-mail » retiré du template, avec `setMailNotif` et
      les trois références à `state.useMailNotification`
- [ ] Sélecteur de fuseau retiré, avec `state.scheduleTimezone`
- [ ] `use_mail_notification` et `schedule_timezone` retirés de `ScheduleConfig`
- [ ] `Timezone` supprimé s'il n'a plus d'utilisateur — le vérifier, ne pas le
      supposer
- [ ] `assets/league_structure.json` nettoyé
- [ ] Non-régression : l'étape 3 enregistre toujours, et une structure existante
      se relit sans erreur
- [ ] `make check-arch` et `make e2e`
