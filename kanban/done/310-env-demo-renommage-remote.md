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

- [x] Renommer `.env.demo` → `.env.remote.demo`
- [x] Mettre à jour `.gitignore:16`
- [x] Mettre à jour le `Makefile` — lignes 43, 183, 190
- [x] Renommer `init_demo_db` → `init_remote_demo_db` : le nom de la cible
      porte le même angle mort que celui du fichier
- [x] Documenter dans `.env.example` que tout fichier visant un hôte distant se
      nomme `.env.remote.*`

## La garde : faite, et elle vise autre chose que ce que la carte croyait

La carte proposait de refuser une cible destructrice visant un hôte distant.
Appliqué à `init_remote_demo_db`, ce serait une cérémonie vide : cette cible
vise **toujours** une base distante, et elle porte déjà ses vérifications de
cohérence et sa double confirmation.

Le vrai danger était ailleurs : **`reset_db` fait `sqlx database reset -y -f`,
sans la moindre confirmation**, sur l'URL du profil courant. Donc
`make reset_db EXEC_PROFILE=remote.demo` détruisait la base distante en
silence — et un `export DATABASE_URL=…distant…` oublié dans le shell suffisait
aussi, la variable d'environnement l'emportant sur le fichier.

La garde est donc posée sur `reset_db` et `reset_test_db`, les cibles
**censées être locales**. Elle refuse tout hôte non local et se contourne par
`I_KNOW_THIS_IS_REMOTE=1`.

```
$ DATABASE_URL="postgres://…@137.74.92.46:5432/kreek_db" make reset_db
  /!\  Refus : cible destructrice sur un hôte distant
     Hôte   : 137.74.92.46
```

**Elle protège quel que soit le nom du profil** — c'est ce qui la rend plus
utile que le renommage, lequel n'informe qu'au moment où l'on écrit la
commande, pas quand on la relance depuis l'historique.

## Le renommage a coûté une ligne

Le `Makefile` avait déjà une source unique — `DEMO_PROFILE := demo` — dont tout
découle. Le reste : `.gitignore`, la doc de `.env.example`, le renommage de la
cible et les quatre commentaires qui la nommaient.

## Carte close
