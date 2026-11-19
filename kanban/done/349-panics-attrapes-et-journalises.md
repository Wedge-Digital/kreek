# `web` + bus — Un panic ne doit pas être l'incident le moins renseigné

**Priorité : moyenne**
**Dépend de :** carte 345 (le span de requête, sans quoi la ligne de panic reste
orpheline)
**Fichiers :** `Cargo.toml`, `src/main.rs`,
`src/web/middleware/panic_response.rs`,
`src/common/services/event_bus/supervision.rs`, les 27 fichiers de listeners et
de publishers

## Le problème — versant HTTP

`tower-http` est compilé avec les features `trace` et `fs` seulement — pas de
`catch-panic`. Un panic dans un handler tue donc la tâche : le client voit une
connexion coupée sans réponse, et le journal reçoit un message de panic brut,
**hors de tout span** — sans identifiant de requête, sans chemin, sans coach.

C'est le pire cas possible : l'incident qui produit le moins d'information est
exactement celui qui en demande le plus. Et comme le client ne reçoit pas de
statut, il n'y a même pas de `500` dans le journal de requêtes pour signaler
qu'il s'est passé quelque chose.

Le projet n'est pas exempt de sources de panic : `unwrap()`, `expect()` et
indexations diverses existent dans le code, y compris sur des chemins de rendu.

## Le problème — versant bus

Le même angle mort existe de l'autre côté, et il est plus insidieux.

Chaque listener et chaque publisher est une tâche `tokio::spawn` tenant une
boucle `loop { rx.recv().await }`. Un panic à l'intérieur fait sortir la tâche
de sa boucle, et **personne n'attend son `JoinHandle`** : le `JoinError` part à
la poubelle. Le BC cesse alors de réagir aux app events **définitivement**,
sans qu'aucune ligne ne le signale.

Côté HTTP, un panic se voit au moins du client. Ici, rien : les projections
cessent de se mettre à jour, et le seul symptôme est une donnée qui ne bouge
plus — constaté des heures plus tard, sans rien dans le journal pour dater
l'incident.

## Ce qu'il faut faire — versant HTTP

Ajouter la feature `catch-panic` à `tower-http` et poser `CatchPanicLayer`
dans le routeur.

**L'ordre des couches est le premier point délicat** : la couche doit se situer
**à l'intérieur** du span de requête, pour que la ligne de panic hérite du
`rid`, du chemin et du coach. Posée à l'extérieur, elle attrape bien le panic
mais le journalise hors contexte — on aurait ajouté un `500` propre sans rien
gagner sur le diagnostic, qui est l'objet de l'épic.

**Le second point délicat n'était pas prévu, et il annulait la carte entière.**
`CatchPanicLayer::new()` journalise via son gestionnaire par défaut, sur la
cible `tower_http::catch_panic`. Or le filtre construit par
`filtre_depuis_config` est `kreek=<niveau>,sqlx=warn` : une cible qui n'est ni
`kreek` ni `sqlx` n'est activée par aucune directive. **La ligne n'existe pas.**
On aurait donc livré un `500` propre et zéro ligne de journal — exactement ce
que la carte prétendait corriger, avec l'apparence du travail fait.

C'est le même piège que la carte 344 avait trouvé sur le `TraceLayer`, muet
pour la même raison sur `tower_http::trace`. Il reparaîtra à chaque couche
tierce branchée en comptant sur sa journalisation intégrée : **une bibliothèque
journalise sur son propre nom, et notre filtre ne connaît que le nôtre.**

D'où `CatchPanicLayer::custom(JournalDePanic)` : un gestionnaire maison qui
émet depuis `web::middleware::panic_response`, donc sous la cible `kreek`. En
prime, un champ nommé (`panic=…`) plutôt qu'un message formaté.

La réponse renvoyée est un `500` en texte brut. Pas de fragment HTMX élaboré :
un panic n'est pas un cas métier, et rien ne garantit que l'état de
l'application permette encore de rendre quoi que ce soit de sensé.

## Ce qu'il faut faire — versant bus

Un `spawn_listener(module_path!(), …)` qui remplace `tokio::spawn` dans les
`init()`, enveloppe la souscription dans un `catch_unwind` et journalise une
ligne `ERROR` nommant le listener disparu.

`module_path!()` sur tous les sites d'appel : le chemin du module désigne le
listener sans qu'on ait à le nommer une seconde fois, et il ne peut pas diverger
du code.

**Ça rend la mort bruyante, ça ne ressuscite personne.** Reprendre la
souscription supposerait de reconstruire la boucle — donc de recloner ses
dépendances et de se réabonner, le `rx` en cours étant consommé. C'est un cran
de plus, qui ne se justifiera que si des panics se produisent réellement :
aujourd'hui, le manque est qu'on ne le saurait même pas.

## Le piège du `downcast`

`Box<dyn Any + Send>` est **elle-même** `Any`. Écrire `message_du_panic(&err)`
produit donc un `&dyn Any` qui désigne la boîte et non son contenu : les deux
`downcast_ref` échouent systématiquement, et tous les panics se journalisent
« message illisible ». Le compilateur ne dit rien — les deux formes typent.
Il faut `err.as_ref()`.

Le bug a été écrit puis attrapé par le test ci-dessous, ce qui est la meilleure
justification qu'on puisse donner de son existence.

## Ce qui reste hors périmètre, et pourquoi

Le `tokio::spawn` de `spaces/io/app_events/user_created_listener.rs` n'est pas
supervisé : c'est une tâche **par événement**, pas la boucle de souscription. Un
panic dedans perd un événement, il ne tue pas le listener, et le message de
`spawn_listener` — « plus aucun événement ne lui parviendra » — y serait faux.

## Checklist

- [x] Feature `catch-panic` ajoutée à `tower-http`
- [x] `CatchPanicLayer` posé **à l'intérieur** du span de requête
- [x] Gestionnaire de panic maison, émettant sous la cible `kreek` — sans quoi
      la ligne est filtrée et la carte ne livre rien
- [x] Test : sous le filtre **réellement construit** par `filtre_depuis_config`,
      la ligne du panic est bien émise, et une ligne émise sur
      `tower_http::catch_panic` est bien perdue
- [x] Test : un panic devient une réponse `500` plutôt qu'une connexion coupée
- [x] Test : les deux formes de charge de panic (`&str` et `String`) sont
      lisibles
- [x] `spawn_listener` sur les 28 sites de souscription — 19 listeners,
      7 publishers, `event_log_feeder`
- [x] Test : un listener qui panique laisse une ligne `ERROR` qui le nomme, et
      une souscription terminée normalement n'en laisse pas
- [x] `make lint`, `make test` et `make check-arch` passent
- [x] Vérifié en conditions réelles, sur une route jetable retirée depuis :
      un `500` côté client, et les deux lignes attendues sous le même `rid` —
      `ERROR … panic dans un handler — requête abandonnée panic=boum — …`
      puis `INFO … ← réponse envoyée status=500 Internal Server Error`
