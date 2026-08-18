# E11 — Savoir ce qui se passe en production

**État :** 6 cartes · 1 faite — la 344 est livrée et vérifiée

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
| 346 | Le niveau rejoint la configuration | `APP__LOG__LEVEL`, défaut `info`, `sqlx` enfin audible |
| 347 | `Debug` sur les commandes, et trois secrets à masquer | prérequis de la 348, et suppression d'une fuite déjà possible |
| 348 | Chaque use case dit ce qu'on lui a demandé | le chemin nominal existe enfin dans le journal |
| 349 | Un panic ne doit pas être l'incident le moins renseigné | un `500` propre et une ligne de journal dans le contexte de la requête |

## Ce qui commande l'ordre

**344 d'abord, seule.** C'est le seul correctif du lot, il tient en quelques
lignes de `main.rs`, et il rétablit à lui seul le plus grand écart. Livrable
immédiatement.

**345 ensuite**, parce que tout le reste en dépend pour être exploitable : sans
`rid`, les lignes des cartes suivantes ne se rattachent à rien. C'est aussi la
carte qui apporte le plus de valeur par unité d'effort — un span de requête fait
hériter le contexte aux **198 `error!` existants sans en modifier un seul**.

**347 avant 348, impérativement.** La 348 journalise la commande reçue par
chaque use case ; trois commandes de `auth` portent un mot de passe ou un jeton
en clair. Prendre les cartes dans le mauvais ordre publie des secrets dans
`docker logs`.

**346 et 349 sont indépendantes** et peuvent s'intercaler n'importe où après la
344.

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
