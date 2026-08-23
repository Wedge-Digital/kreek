# E02 — Notifications e-mail de compétition

**État :** en cours — 11 cartes · 7 faites. Démarrée le 2026-08-22.
La 331 pose le modèle et la colonne ; la 332 donne le premier chemin
d'édition — un organisateur peut régler les notifications d'une compétition
déjà démarrée, ce qu'aucun écran ne permettait.
**Spec :** `docs/specs/notifications/` (`configuration/` et `envoi/`)

## La fonction

Un coach est prévenu par e-mail de ce qui le concerne dans sa compétition :
l'ouverture des inscriptions, la veille d'une journée, la fin d'une journée, et
l'approche de la date limite d'inscription. Un organisateur choisit lesquelles
de ces quatre notifications sont actives, à la création de la compétition comme
après son démarrage.

L'application porte aujourd'hui **deux interrupteurs e-mail qui ne branchent
rien** (`use_mail_notification`, `notify_by_email`). L'épic les remplace par un
réglage qui fait réellement partir des e-mails — et retire les deux morts.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 331 | Domaine et persistance des réglages | **faite** — colonne JSONB `notifications`, `applicability()` |
| 332 | Widget de réglage + hôte admin | **faite** — premier chemin d'édition, mode auto-save |
| 333 | Le même widget dans le magicien | **faite** — mode différé, `notify_by_email` retiré |
| 334 | Retrait des trois réglages morts | **faite** — le volet `configuration/` est clos |
| 335 | Journal d'envois — table et repository | **faite** — `claim`/`confirm`, l'idempotence de R3 |
| 336 | Domaine de l'ordonnancement — `due_today()` | **faite** — ce qui part aujourd'hui, pur et testable |
| 337 | Résolution des destinataires | **faite** — qui reçoit quoi, borné par l'espace (R7) |
| 338 | Les quatre gabarits d'e-mail | les maquettes validées, en Askama |
| 339 | Le cœur d'expédition | réserver, rendre, envoyer, confirmer |
| 340 | Les deux déclencheurs — CLI du cron et listener d'ouverture | **c'est elle qui allume la fonctionnalité** |
| 325 | L'e-mail de mot de passe au nouveau standard visuel | harmonisation, pas une réécriture |

## Ce qui commande l'ordre

C'est le seul chantier du backlog dont l'ordre est imposé de bout en bout.

```
331 ─┬─► 332 ─► 333 ─► 334
     └─► 336 ─────────────┐
335 ─────────────┐        │
337 ─┬─► 338 ────┼─► 339 ─┴─► 340
     └───────────┘
```

**Rien ne part avant la 340.** Les dix cartes précédentes portent toutes le
même avertissement en exergue : livrer `configuration/` sans `envoi/`
produirait un **troisième interrupteur e-mail mort**, mieux dessiné que les
deux qu'il remplace et tout aussi inerte — le défaut même que la fonctionnalité
corrige.

La 325 est indépendante des dix autres : elle ne dépend que des maquettes
validées en phase 1, et peut se faire à tout moment.

## À trancher avant la 340

Les saisons antérieures à la migration de la 331 ont `notifications` à `NULL`,
donc **démarrent allumées** — la carte prévoyait de les remplir à `false`, et il
a été décidé de ne pas le faire. Sans effet tant que rien n'envoie ; le jour où
la 340 allume la fonctionnalité, 213 saisons deviennent notifiantes d'un coup.
Assumer, ou rattraper la donnée : la question se pose avant cette carte-là, pas
après.

## Ce que l'épic ne couvre pas

- **Toute notification hors compétition** — rien sur les équipes, les joueurs
  ou les espaces.
- **Une préférence de désabonnement par coach.** Le réglage est par saison, pas
  par destinataire.
- **L'internationalisation.** La carte 325 tranche : le français seul, et
  supprime le dossier `en_EN/` que rien ne lisait. Si l'anglais revient, il
  faudra d'abord une préférence de langue par coach, que ni `auth__users` ni
  `spaces__user_cache` ne portent.
- **Le rattrapage des envois perdus.** R9 l'interdit explicitement : une ligne
  de journal restée à `NULL` est un échec constaté, jamais rejoué le lendemain.

## Terminé quand

Un coach inscrit à une compétition reçoit, la veille d'une journée, un e-mail
listant ses matchs — sans que personne n'ait lancé de commande à la main, et
sans qu'une seconde exécution du cron le même jour lui en envoie un second.
