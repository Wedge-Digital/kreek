# E02 — Notifications e-mail de compétition

**État :** close le 2026-08-25 — 13 cartes · 13 faites. Démarrée le 2026-08-22.
Les 430 et 431, nées de la revue de déploiement du 2026-08-24, ont rendu le
critère de clôture ci-dessous **constaté** ; la 338 l'a complété par le seul
contrôle qu'aucun test ne peut faire.
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
| 338 | Les quatre gabarits d'e-mail | **faite** — et le rendu réel a trouvé `http:///`, un `HOST_DOMAIN` vide que rien ne signalait |
| 339 | Le cœur d'expédition | **faite** — réserver, rendre, envoyer, confirmer |
| 340 | Les deux déclencheurs — CLI du cron et listener d'ouverture | **faite — la fonctionnalité est allumée** |
| 325 | L'e-mail de mot de passe au nouveau standard visuel | **faite** — harmonisation, pas une réécriture |
| 430 | Éteindre les saisons antérieures à la migration | **faite** — 318 saisons éteintes, colonne `NOT NULL` avec défaut « neuve » |
| 431 | Tests du chemin du cron | **faite** — 5 unitaires sur le vrai SQL, 3 e2e à travers le binaire |

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

## Ce que l'épic a laissé passer, et comment

Les deux manques ci-dessous se lisaient dans ce document depuis la carte 340,
sans figurer à son tableau. **Aucun ne s'est vu avant qu'on prépare un
déploiement** — un état écrit dans le corps d'une épic mais absent de sa liste de
cartes n'est pas un état suivi. C'est la leçon à retenir de l'épic, plus que les
deux corrections elles-mêmes.

**La question posée « avant la 340 » ne l'a pas été.** Les saisons antérieures à
la migration de la 331 ont `notifications` à `NULL`, donc démarrent **allumées**,
contre R8. La 331 comptait les remplir à `false` ; il a été décidé de ne pas le
faire, en renvoyant la décision avant la 340. La 340 est passée sans que
personne ne la reprenne.

Elles étaient 318 sur 471 — l'épic annonçait 213, la spec ~399. Les trois
chiffres diffèrent, ce qui dit assez que la question a dormi. Tranchée par la
**carte 430** : éteintes, et la colonne passe `NOT NULL` avec un défaut « tout
allumé » pour les neuves, de sorte que le cas ne puisse plus réapparaître.

**Le chemin du cron n'avait aucun test.** Les deux extrémités en avaient — 17 sur
`due_today()`, 6 sur l'expédition — mais les 180 lignes qui les cousent, aucune,
ni unitaire ni e2e. Elles portent trois des quatre notifications. Couvert par la
**carte 431**, qui a au passage mis au jour un mode d'échec de plus :
`traiter_saison()` avale l'erreur du dépôt de journées par `unwrap_or_default()`,
transformant une panne en « rien à envoyer », sans une ligne de journal.

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

**Constaté** par `tests/e2e/test_notification_cron.py`, qui construit deux
équipes inscrites, programme une journée au lendemain, invoque le binaire, et lit
la table du journal. Les trois tests échouent si la journée est décalée d'un
jour — vérifié, et c'est ce qui distingue un critère atteint d'un critère
supposé.

La 338 a complété ce constat par ce qu'aucun test ne peut dire : ce que ces
e-mails **donnent à voir**. Les trois notifications du cron ont été rendues par
la commande de production elle-même, et le premier rendu a immédiatement montré
un `<img src="http:///…">` — `AppConfig::app_url()` transforme un domaine vide
en `"http://"` au lieu de refuser, et `.env.dev` comme `.env.remote.demo` l'ont
vide.

**L'ouverture dans Outlook et Gmail n'a pas eu lieu** : elle demandait un envoi
réel depuis une clé Resend, et le sujet a été jugé maîtrisé. L'épic est close
sans cette dernière observation, et il vaut mieux que ce soit écrit qu'oublié.

## Ce que l'épic laisse derrière elle

Un défaut latent, hors de son périmètre et sans carte à ce jour : `app_url()`
rend une URL sans hôte plutôt que d'échouer quand `HOST_DOMAIN` est vide. Il ne
touche aujourd'hui que dev et démo. Le jour où un déploiement oublie ce réglage,
tous ses e-mails partiront avec des liens morts, sans une ligne de journal.
