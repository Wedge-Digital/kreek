# E11 — Savoir ce qui se passe en production

**État :** 8 cartes · 7 faites — 344 à 350 livrées et vérifiées ; reste 351

## La fonction

En production, on ne sait pas ce qui se passe. Le comptage des appels à
`tracing` explique pourquoi :

| Niveau | Occurrences |
|---|---|
| `error!` | 198 |
| `warn!` | 74 |
| `info!` | 12 |
| `debug!` | 16 |

Un journal qui ne parle qu'en cas d'échec ne répond pas à « que s'est-il passé ? »,
il répond à « qu'est-ce qui a cassé ? ». Les deux bugs du mode customisation en
sont la démonstration : rien n'échouait, le comportement était simplement faux,
et aucune ligne n'aurait mis sur la voie.

Deux découvertes aggravent le tableau, et ce sont elles qui font de la première
carte un correctif plutôt qu'une amélioration :

- **`request_log` est compilé hors du binaire release** — il n'est branché qu'à
  l'intérieur d'un bloc `#[cfg(debug_assertions)]` ;
- **le `TraceLayer` est muet** — il émet sur `tower_http::trace`, que le filtre
  par défaut `kreek=debug` n'active pas.

Donc en production, chaque ligne du journal est un `error!` isolé : sans
méthode, sans chemin, sans statut, sans utilisateur. On ne peut même pas savoir
quelle requête l'a provoqué.

L'épic rend le journal capable de raconter une requête de bout en bout, dans
`docker logs`, avec `grep` pour seul outil.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 344 | Le journal de production n'existe pas | un journal de requêtes, toujours actif, sans doublon ni champ trompeur |
| 345 | Un identifiant sur chaque ligne, et de quoi le retrouver | `rid` hérité par toutes les lignes, écho en `x-request-id`, durées |
| 346 | Le niveau rejoint la configuration | `LOG__LEVEL`, défaut `info`, `sqlx` enfin audible |
| 347 | `Debug` sur les commandes, et trois secrets à masquer | prérequis de la 348, et suppression d'une fuite déjà possible |
| 348 | Chaque use case dit ce qu'on lui a demandé | le chemin nominal existe enfin dans le journal |
| 349 | Un panic ne doit pas être l'incident le moins renseigné | un `500` propre et une ligne de journal dans le contexte de la requête |
| 350 | Le versant émission des app events entre dans le journal | un `grep event_id=` rend l'émission **et** toutes les réactions |
| 351 | Remonter d'une réaction à la requête qui l'a causée | *à raffiner* — le `rid` ne franchit pas le `tokio::spawn` |

Les deux dernières sont nées de la 345 : instrumenter les 19 listeners a montré
ce qui manquait de l'autre côté du bus.

## Ce qui commande l'ordre

**344 puis 345 — faites.** La première était le seul correctif du lot ; la
seconde apportait le plus de valeur par unité d'effort, un span de requête
faisant hériter le contexte aux **198 `error!` existants sans en modifier un
seul**.

**346 — faite** aussi. Elle a révélé au passage que la forme `APP__<SECTION>__<CLÉ>`
documentée dans `CLAUDE.md` était **inopérante** : `config::Environment::default()`
ne pose aucun préfixe, et une variable ainsi nommée était ignorée en silence.

L'ordre des cinq restantes, du moins cher au plus engageant :

**349 — faite.** Petite, indépendante, à effet immédiat en production : elle
supprime le pire angle mort, l'incident qui produit le moins d'information
alors qu'il en demande le plus. Elle a été élargie au versant bus — un listener
qui panique mourait en silence, et le BC cessait de réagir sans qu'aucune ligne
ne le signale. Elle a aussi retrouvé le piège de la 344 sous une autre forme :
`CatchPanicLayer::new()` journalise sur `tower_http::catch_panic`, cible que le
filtre `kreek=…` n'active pas — la couche aurait été livrée muette.

**347 — faite.** Elle devait précéder la 348 impérativement : celle-ci
journalise la commande reçue par chaque use case, et trois commandes de `auth`
portaient un mot de passe ou un jeton en clair. Le masquage s'est fait par un
newtype `Secret<T>` plutôt que par des `Debug` écrits à la main — un `Debug`
manuel ne protège que les champs auxquels on a pensé le jour où on l'a écrit.
Elle a aussi montré que `spaces` avait son dossier de use cases mal
orthographié, angle mort de tout script visant `use_cases/`.

**348 — faite.** Le gros morceau de l'épic et sa raison d'être : c'est elle qui
fait exister le chemin nominal. Elle a failli ne rien livrer du tout —
`#[instrument]` crée un span mais n'émet aucun événement, et sur des use cases
muets il n'aurait produit aucune ligne. La ligne vient d'une couche dédiée aux
spans de `use_cases/`, ce qui laisse intact le choix de la 345 de ne pas
activer `FmtSpan` globalement.

**350 — faite.** Elle complétait le tableau des app events, et ne pouvait pas
précéder la 348 : tant que les use cases étaient muets, la moitié de ce qu'elle
relie n'existait pas. Elle a fait naître la carte 352 — deux use cases de
`match_report` émettent un app event directement, le publisher ne traitant pas
`MatchReportConfirmed`.

**351 en dernier, et seulement après raffinage.** C'est la plus invasive — elle
touche `shared_kernel` — et sa valeur croît avec le nombre de spans à relier :
la faire tôt reviendrait à câbler une corrélation entre deux points dont l'un
est encore vide.

Vérification préalable, sans code : la rotation du pilote de logs Docker
(`max-size`, `max-file`). Elle est portée par la 346. Sans elle, tout le reste
est vain — le meilleur journal du monde ne vaut rien s'il s'évapore à la
recréation du conteneur.

## Ce que l'épic ne couvre pas

**Le reclassement des 198 `error!`.** Beaucoup n'en sont pas — une entrée
introuvable, une saisie invalide. Quand tout est une erreur, plus rien n'en est
une. Le chantier est réel mais long, et le mélanger à l'ajout de traces
rendrait les deux illisibles. À faire dans une carte à part, plus tard.

**Le JSON et l'agrégation.** Le besoin est `docker logs` et `grep` en terminal.
Le format texte reste le bon choix, et il impose ses règles : une ligne par
événement, pas de `pretty()`.

**L'audit.** « Qui a modifié quelle équipe, quand » ne relève pas du journal
applicatif : l'event store le donne déjà, et le dupliquer dans les logs
coûterait sans rien apporter.

**Le bus de commandes et le runner de use cases.** Les deux ont été étudiés
puis écartés, et la raison mérite d'être écrite ici pour ne pas être
re-débattue :

- un **bus de dispatch** effacerait le graphe d'appel statique, or
  `check-arch` est un ensemble de `grep` — on perdrait la prise de l'outil de
  vérification qu'on a délibérément choisi. Il imposerait aussi un type d'erreur
  commun là où il en existe 44 distincts, et ferait passer l'échec de dispatch
  de la compilation à l'exécution ;
- le **middleware transactionnel**, seconde justification du bus, ne tient pas :
  une passe sur les 63 use cases n'a trouvé que **deux** écritures multi-dépôts
  (`create_draft_competition` et `validate_customisation`), toutes deux
  intra-BC. Deux corrections ciblées, pas un *unit of work* ;
- un **runner générique** décorerait l'appelant : 49 sites d'appel à modifier,
  et un de plus à chaque nouveau use case, qui pourrait l'oublier.
  `#[instrument]` décore l'appelé — impossible à contourner, vérifiable par une
  adjacence de deux lignes dans un seul dossier.

Le choix reste réversible sans dette : si une seconde préoccupation transverse
apparaît, ajouter un runner plus tard n'obligera pas à retirer les attributs —
les spans s'emboîtent.

## Terminé quand

Au prochain incident signalé par un coach, on part de l'en-tête
`x-request-id` lu dans son navigateur, on fait un `docker logs … | grep rid=…`,
et on reconstitue ce qu'il a tenté et pourquoi ça a échoué **sans ouvrir le
code ni interroger la base**.
