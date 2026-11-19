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

## Ce que la réalisation a établi

**La version est 0.15.0, pas 0.14.** `tower-sessions-sqlx-store` 0.14.x dépend
de `tower-sessions-core` 0.13 ; c'est la 0.15.0 qui suit le `tower-sessions`
0.14 du projet.

**La table est la nôtre** — `migrations/20261102000002_sessions.sql`, table
`sessions` dans `public`, sans préfixe de BC : elle appartient à l'hôte, comme
`event_log`. `PostgresStore::migrate()` n'est pas appelé : il crée un schéma
`tower_sessions` (droit `CREATE SCHEMA` exigé) et échappe au `MIGRATOR`.

**La table ne contient pas les cookies.** L'identifiant de session est la valeur
même du cookie, non signée. Le décorateur `HashedIds` range chaque session sous
l'empreinte SHA-256 (tronquée à 128 bits) de son identifiant : une ligne volée
ne se rejoue pas. Le dump de production exclut de toute façon les lignes de la
table (`--exclude-table-data=sessions`) : chacune porte une copie de
l'empreinte du mot de passe de son coach (`session_auth_hash`).

**Base indisponible : accepté, et journalisé.** Une requête qui passe par la
couche de session rend un 500 ; seuls `/static` et `/css` restent servis. Le 500
est écrit par `tower_sessions` et `axum_login` sur leurs propres cibles, hors du
filtre `kreek=…` : le décorateur `Logged` journalise donc chaque échec du
magasin sous `kreek::common::session_store`.

**La purge** est une boucle à nous, horaire, qui survit à ses échecs (`warn`,
puis nouvel essai) — `continuously_delete_expired` s'arrête au premier.

**La fenêtre ne glisse pas** — défaut hérité de la carte 490, hors de cette
carte. Sans `with_always_save`, `tower-sessions` ne réécrit ni la session ni le
cookie tant que rien ne la modifie : la session expire trente jours après la
connexion. Le commentaire de `main.rs` qui affirmait le contraire est corrigé ;
la correction fera l'objet d'une carte à part.

## Le déploiement

**Tout le monde est déconnecté une dernière fois.** Les sessions de l'ancien
processus vivent dans sa mémoire ; la nouvelle table part vide. Les
déploiements suivants les conservent. À annoncer aux coachs.

**Avant** : rien en base — la migration s'applique au démarrage, avec le rôle
qui applique déjà toutes les autres.

**Après** :

```sql
SELECT count(*), min(expiry_date), max(expiry_date) FROM sessions;
```

La table se remplit ; un redémarrage laisse connecté ; au journal, la ligne
`purge des sessions expirées programmée` ; au-delà de trente jours,
`min(expiry_date)` reste postérieur à `now() - interval '1 hour'`.

**Le retour arrière** : `run_migrations` refuse de démarrer quand une migration
appliquée manque au binaire. Revenir au binaire précédent après ce déploiement
échoue donc au démarrage, sauf à retirer d'abord la ligne
`20261102000002` de `_sqlx_migrations` (la table `sessions` peut rester, ou
être supprimée). Les coachs sont alors déconnectés une fois de plus.
