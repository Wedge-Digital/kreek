# E02 — Notifications e-mail de compétition

**État :** en cours — 13 cartes · 10 faites. Démarrée le 2026-08-22.
Restent la 338 (vérification en client réel), la **366** (R8 jamais tranchée) et
la **367** (le chemin du cron sans test). Les deux dernières sont nées de la
revue de déploiement du 2026-08-24 : l'épic les nommait déjà dans son corps sans
les avoir inscrites au tableau.
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
| 339 | Le cœur d'expédition | **faite** — réserver, rendre, envoyer, confirmer |
| 340 | Les deux déclencheurs — CLI du cron et listener d'ouverture | **faite — la fonctionnalité est allumée** |
| 325 | L'e-mail de mot de passe au nouveau standard visuel | **faite** — harmonisation, pas une réécriture |
| 366 | Éteindre les saisons antérieures à la migration | R8, restée en suspens alors que la 340 est passée |
| 367 | Tests du chemin du cron | trois notifications sur quatre, sans aucun filet |

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

## Ce que l'épic a laissé passer

**La question posée « avant la 340 » ne l'a pas été.** Les saisons antérieures à
la migration de la 331 ont `notifications` à `NULL`, donc démarrent **allumées**,
contre R8. La 331 comptait les remplir à `false` ; il a été décidé de ne pas le
faire, en renvoyant la décision avant la 340. La 340 est passée sans que
personne ne la reprenne.

Elles sont 318 sur 471 — l'épic annonçait 213, la spec ~399. Les trois chiffres
diffèrent, ce qui dit assez que la question a dormi. C'est la **carte 366**.

**Le chemin du cron n'a jamais reçu de test.** Les deux extrémités en ont — 17
sur `due_today()`, 6 sur l'expédition — mais les 180 lignes qui les cousent,
aucune, ni unitaire ni e2e. Elles portent trois des quatre notifications, et le
critère de clôture ci-dessous parle précisément d'elles. C'est la **carte 367**.

Les deux se voient à la lecture de cette épic depuis le jour de la 340. Aucune ne
s'est vue avant qu'on prépare un déploiement — un état écrit dans le corps d'une
épic mais absent de son tableau n'est pas un état suivi.

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

**Ce critère vise le chemin du cron, que rien n'exerce aujourd'hui** (carte 367).
Tant qu'il n'est pas couvert, l'épic ne peut pas se déclarer close : on le
supposerait atteint sans l'avoir constaté, ce que la règle des épics interdit.
