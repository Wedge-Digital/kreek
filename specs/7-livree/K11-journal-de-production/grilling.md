# K11 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de l'épic `kanban/epics/done/E11-journal-de-production.md` (devenue `initiative.md`). Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K11-B1** — Terminé quand (critère de l'épic E11) : À partir d'un `x-request-id` prélevé sur **une action quelconque** de l'application, on reconstitue dans `docker logs`, **sans ouvrir le code ni interroger la base** : 1. ce que le coach a demandé — la commande reçue, champs compris ; 2. ce que le système en a fait — les use cases traversés et leur durée ; 3. ce qu'il a émis — les domain events, avec leurs identifiants ; 4. ce qui a franchi les frontières de BCs — les app events, reliés aux domain events dont ils sont issus ; 5. comment chaque BC y a réagi. Trois `grep` enchaînés suffisent. ### Pourquoi le critère a été réécrit Il disait : « **au prochain incident signalé par un coach**, on part de son `x-request-id` […] et on reconstitue ce qu'il a tenté et pourquoi ça a échoué ». Ce critère n'était pas observable à volonté : il attendait qu'un incident se produise. Il confondait deux choses — **la capacité existe** et **l'occasion s'est présentée** — dont seule la première dépend de nous. Une épic dont la clôture dépend d'un événement extérieur reste ouverte indéfiniment sans qu'aucun travail ne reste à faire, ce qui vide la colonne `en_cours/` de son sens. Il portait aussi une hypothèse fausse : « **pourquoi ça a échoué** ». Les deux bugs du mode customisation qui ont motivé cette épic **n'échouaient pas** — rien ne cassait, le comportement était simplement faux. Un critère qui ne parle que d'échec ne couvre pas le cas qui a fait naître l'épic. La version ci-dessus se constate quand on veut, sur n'importe quelle action, et ne présuppose pas qu'il se soit passé quelque chose de mal.
- **K11-B2** — `web` — En production, le journal de requêtes n'existe pas (ticket `344-journal-de-production-existe`)
- **K11-B3** — `web` — Un identifiant sur chaque ligne, et de quoi le retrouver (ticket `345-span-de-requete-et-identifiant`)
- **K11-B4** — `config` — Le niveau de journalisation rejoint la configuration (ticket `346-niveau-de-log-dans-appconfig`)
- **K11-B5** — `app` — `Debug` sur les commandes, et trois secrets à masquer (ticket `347-debug-sur-les-commandes-et-secrets`)
- **K11-B6** — `app` — Chaque use case dit ce qu'on lui a demandé (ticket `348-instrument-sur-les-use-cases`)
- **K11-B7** — `web` + bus — Un panic ne doit pas être l'incident le moins renseigné (ticket `349-panics-attrapes-et-journalises`)
- **K11-B8** — `app` — Le versant émission des app events entre dans le journal (ticket `350-publishers-app-events-instrumentes`)
- **K11-B9** — `common` — Remonter d'une réaction à la requête qui l'a causée (ticket `351-relier-une-reaction-a-sa-requete`)
- **K11-B10** — `check-arch` + `CLAUDE.md` — les verrous d'observabilité couvrent la règle, pas la forme (ticket `355-verrous-d-instrumentation-sans-trou`)

## Hors périmètre

Voir la section « Ce que l'épic ne couvre pas » de `initiative.md`, s'il y en a une.
