# Phase 6 — Domaine : sans objet pour cette unité

**Cette unité n'ajoute rien au domaine.** Le contenu de la phase 6 est celui de
`onglet-presences/06-domaine.md`, et il couvre déjà les trois unités.

## Pourquoi cette page existe quand même

Une case vide dans le tableau de progression laisse croire à un trou ; une case
renseignée explique. C'est la raison qui a fait écrire la ligne
« `reponse-coach` — phase 2 : sans objet » dans le README, et elle vaut ici.

## Ce qui était déjà là

L'agrégat a été **conçu d'un bloc** en phase 6 de l'unité 1, le workflow le
demandant explicitement : *la troisième méthode qu'on greffe révèle souvent que
les deux premières avaient la mauvaise signature*. Concrètement, il devait savoir
répondre dès sa conception à « un coach répond depuis un jeton », et il le sait :

| Ce que cette unité appelle | Posé par |
|---|---|
| `reponse_par_jeton(token)` | carte 511 |
| `enregistrer(team, venue, Repondant::Coach, journee, maintenant)` | carte 512 |
| `statut(maintenant)` — R7, la validité du jeton se lit sur la campagne | carte 511 |
| `Presence`, `Venue`, `Repondant`, `SurveyToken` | carte 511 |

Les refus qu'elle rencontre — `SurveyClosedForCoach` (R21) et
`RoundFrozenByReport` (R13) — sont eux aussi déjà dans `DomainError`.

## La seule chose que cette unité ajoute au domaine

Deux variantes de `NotificationType` (`presence_survey`, `presence_reminder`),
dans `domain/notification_delivery.rs`. C'est un type de journal d'envoi, pas
une règle : il est décrit en phase 3, et figé par le test de valeurs qui protège
déjà les quatre autres.

## Ce que ça prouve, et qu'il vaut la peine de noter

Concevoir l'agrégat d'un bloc, à la fin de l'unité 1, a rendu cette phase-ci
vide. C'était le pari du découpage en unités — et l'inverse aurait été visible
ici : une méthode `enregistrer` conçue pour le seul organisateur n'aurait pas
porté `Repondant`, et cette unité aurait dû la rouvrir pour y ajouter un
paramètre que trois appelants doivent renseigner.
