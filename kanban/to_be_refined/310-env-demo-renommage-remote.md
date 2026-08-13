# `.env.demo` → `.env.remote.demo` — le nom doit dire l'hôte

**Priorité : basse**
**Contexte :** configuration et outillage — `Makefile`, `.gitignore`

## Le problème

`.env.demo` désigne une base sur `137.74.92.46`. Tous les autres fichiers
d'environnement — `.env.dev`, `.env.test`, `.env.legacy` — pointent
`localhost`. **Rien dans le nom ne distingue le seul qui sorte de la machine.**

## Ce que ça a déjà coûté

Pendant la carte 307, la cible `make dev-demo` — qui ne fait que choisir le
**référentiel** de démo (`REFERENCES__DIR=assets/references.example`) — a été
confondue avec `.env.demo`, qui désigne une **base distante**. Des
identifiants d'espace et de joueur ont été lus sur la base distante, puis
proposés comme URLs contre le serveur local, où ils n'existaient pas. La
recherche du « pourquoi ça ne marche pas » a duré plusieurs allers-retours,
tous partis d'un nom qui promettait « démo » et livrait « distant ».

Deux mots voisins, `dev-demo` et `.env.demo`, désignant deux notions sans
rapport : le jeu de règles d'un côté, l'hôte de la base de l'autre.

## Ce qui aggrave

`make init_demo_db` fait un `DROP+CREATE` sur cette base. Une cible
destructrice pointée sur un hôte distant, derrière un nom qui n'annonce que
« démo ». La double confirmation déjà en place protège du geste ; elle ne
corrige pas l'idée fausse que le geste vise la machine locale.

## Portée réelle du renommage

Le fichier est **gitignoré** : le renommer sur un poste ne change rien pour un
autre clone. Ce qui rend la convention effective, ce sont les références
**versionnées**.

- [ ] Renommer `.env.demo` → `.env.remote.demo`
- [ ] Mettre à jour `.gitignore:16`
- [ ] Mettre à jour le `Makefile` — lignes 43, 183, 190
- [ ] Renommer `init_demo_db` → `init_remote_demo_db` : le nom de la cible
      porte le même angle mort que celui du fichier
- [ ] Documenter dans `.env.example` que tout fichier visant un hôte distant se
      nomme `.env.remote.*`

## Question ouverte

Faut-il une **garde d'exécution** en plus du renommage — refuser une cible
destructrice dont l'URL n'est pas `localhost` sans variable explicite
(`I_KNOW_THIS_IS_REMOTE=1`) ?

Le renommage informe ; il n'empêche pas. Un nom se lit au moment où on écrit la
commande, pas au moment où on la relance depuis l'historique du shell.
