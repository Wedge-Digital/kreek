# Les sessions ne survivent pas à un redéploiement

**Priorité : à décider après mesure** — dépend de ce que la carte 490 a réglé
**Dépend de :** 490 (livrée) · **Sans épic**

## Le constat

`src/common/session_store.rs` garde les sessions dans un `DashMap`, et son
en-tête le dit lui-même :

> Drop-in replacement for MemoryStore in **dev/staging**.

**Chaque redémarrage du processus déconnecte tout le monde, en même temps.** Le
`CLAUDE.md` l'annonçait comme une phase 1 provisoire ; la phase 2 n'a jamais eu
lieu.

Deux effets secondaires du même magasin : il **ne nettoie jamais** — `load`
filtre les sessions expirées mais ne les supprime pas, donc la mémoire monte
lentement — et il est instancié dans `build_router`, ce qui lie sa durée de vie
au routeur plutôt qu'au processus.

## Pourquoi cette carte attend

La carte 490 a corrigé deux causes de déconnexion qui n'avaient rien à voir avec
le magasin : un cookie sans durée, et un `SameSite: Strict` qui coupait les
arrivées par lien externe. **On ne sait pas encore ce qu'il restera.**

Si les plaintes cessent, cette carte peut attendre. Si elles persistent **par
vagues corrélées aux déploiements**, c'est elle qu'il faut faire, et vite.

C'est aussi pourquoi il n'y a pas de mesure ici : elle se prend en production,
pas dans le dépôt.

## La correction

`tower-sessions-sqlx-store` — version **0.14**, qui correspond au
`tower-sessions` 0.14 du projet. Postgres, pas Redis : la décision est prise et
le `CLAUDE.md` la porte désormais.

Le changement est localisé : le type passé à `SessionManagerLayer::new(...)`, une
migration pour la table, et la suppression — ou la conservation pour les seuls
tests — de `DashMapStore`.

## Ce qu'il faudra vérifier

**Qu'une session survive vraiment à un redémarrage.** C'est le seul test qui
compte, et il échoue aujourd'hui par construction : se connecter, reconstruire le
routeur sur la même base, rejouer le cookie, et obtenir autre chose qu'une page
de connexion.

**Que les sessions expirées disparaissent.** Le magasin actuel ne nettoie pas ;
un magasin SQL sans purge ferait grossir une table au lieu d'un `DashMap`. La
crate fournit une tâche de nettoyage — il faut la brancher, et vérifier qu'elle
tourne.

**Que le démarrage ne dépende pas de la base pour servir les pages publiques.**
Aujourd'hui un magasin en mémoire ne peut pas échouer ; demain, une base
indisponible pourrait empêcher toute session d'être lue. Le comportement attendu
dans ce cas mérite d'être décidé avant, pas découvert un soir de panne.

## Ce que la carte ne fait pas

**Elle ne change pas la durée ni les attributs du cookie** — c'est fait, carte
490.

**Elle n'introduit pas Redis**, dans aucun cas.
