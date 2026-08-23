# E12 — Administrer les membres d'un espace

**État :** 21 cartes · 0 faite. Conception close — les sept phases du workflow
« Nouvelle fonctionnalité » sont écrites pour les deux onglets, dans
`docs/specs/space-admin/`.

## La fonction

Un administrateur d'espace n'a **aucun écran**. Il ne peut ni voir ses membres,
ni changer un rôle, ni retirer quelqu'un, ni faire entrer un coach. Tout ce qui
relève de l'administration d'un espace se fait aujourd'hui en base.

L'épic livre les deux onglets qui le lui donnent — **Membres** et **Ajout
direct** — sur une page d'administration qui accueillera les deux autres.

## Les cartes

### Préalables

| # | Intitulé | Apport |
|---|---|---|
| 364 | Deux événements de `spaces` portent le même type | `USER_INVITED_IN_SPACE` et `USER_SUBSCRIBED_TO_SPACE` valent la même chaîne |
| 375 | L'agrégat `Space` se charge incomplet, et hors de sa souveraineté | un membre sans avatar en disparaît ; le SQL joint `auth__users` |

### Onglet Membres, et la page qui l'accueille

| # | Intitulé |
|---|---|
| 365 | L'appartenance à un espace devient un invariant |
| 366 | Le dépôt sait lire, modifier et retirer une appartenance |
| 367 | Changer le rôle et retirer un membre, côté applicatif |
| 368 | La page d'administration et sa barre d'onglets |
| 369 | La liste des membres |
| 370 | Les statistiques de l'espace |
| 371 | Changer un rôle, retirer un membre, depuis l'écran |
| 372 | Le bouton de réinitialisation de mot de passe |
| 373 | `competitions` réagit au retrait d'un membre |
| 374 | La page d'administration sous Playwright |

### Onglet Ajout direct

| # | Intitulé |
|---|---|
| 376 | L'ajout par un administrateur devient une commande du domaine |
| 377 | Le dépôt sait chercher dans l'annuaire de la plateforme |
| 378 | Ajouter un membre, côté applicatif |
| 379 | `auth` sait créer un compte sans mot de passe |
| 380 | Le widget de création de compte, fourni par `auth` |
| 381 | L'onglet Ajout direct : le cadre, la recherche, les candidats |
| 382 | Ajouter un coach déjà inscrit |
| 383 | Créer un compte et ajouter |
| 384 | L'onglet Ajout direct sous Playwright |

## Ce qui commande l'ordre

**Les deux préalables d'abord, et seuls.** Ils corrigent des défauts
préexistants ; les mêler à la fonctionnalité produirait un commit où plus
personne ne sait lequel des deux a cassé quoi.

**La 375 mérite d'être lue pour ce qu'elle enseigne.** `find_by_id` du BC
`spaces` n'a **aucun appelant** hors de son propre test : l'agrégat `Space` est
construit et jamais chargé. Deux défauts y dorment depuis toujours — un membre
sans avatar en est silencieusement absent, et sa requête franchit la frontière
du BC `auth`.

La première fonctionnalité à s'en servir les aurait hérités en silence. Et le
symptôme aurait été le pire possible : l'invariant du dernier administrateur ne
serait pas tombé en panne, il aurait **répondu faux** — un espace aurait pu
perdre son dernier administrateur en passant par une garde qui a l'air de
fonctionner.

Elle a été trouvée en préparant la phase 6, par l'étape « présenter l'agrégat
avant de l'écrire » ajoutée au workflow ce jour-là.

**Le domaine avant tout le reste**, puis le dépôt, puis les use cases, puis
l'écran. Les deux onglets suivent la même chaîne, et celle de l'Ajout direct
reprend l'agrégat que celle de Membres a mis en place.

**`auth` est indépendant.** Les cartes 379 et 380 ne dépendent de rien et
peuvent se faire à tout moment. La 380 est une carte à part **bien qu'elle
n'ait aucune valeur** tant que `spaces` ne l'affiche pas : elle vit entièrement
dans `auth`, et un BC dont on maintient l'indépendance ne se livre pas dans le
même commit que son consommateur.

## Ce que l'épic ne couvre pas

- **L'onglet Invitations.** Rien n'existe : ni table, ni jeton, ni durée de
  validité, ni états. C'est le plus gros des quatre, et c'est lui qui rendra
  visible le défaut que la carte 364 corrige par anticipation.
- **L'onglet Paramètres.** Nom et logo existent sur l'agrégat sans moyen de les
  modifier ; la visibilité est neuve de bout en bout — colonne, migration,
  filtrage de l'annuaire `/app/space/all`. C'est lui qui débloquera le badge de
  visibilité que la carte 368 laisse volontairement de côté.
- **La zone de danger** — transférer la propriété, archiver, supprimer. Sortie
  dès la phase 1, et pas pour sa taille : supprimer un espace veut dire
  supprimer ses équipes et ses compétitions, c'est-à-dire commander la
  destruction de données dont d'autres BCs sont souverains. Aucune des trois
  opérations n'a d'ailleurs d'objet auquel s'appliquer — ni propriétaire, ni
  état d'archivage n'existent en base. **Épic dédiée.**

## Ce qu'elle apprend au passage

**Un contrat non typé entre deux BCs.** `accountCreated`, `coach_id`, `name` :
trois chaînes qui franchissent la frontière entre `auth` et `spaces` **par le
navigateur**. Ni le compilateur, ni `cargo test`, ni `check-arch` — un `grep`
aveugle aux chaînes littérales et aux attributs HTML — ne les voient.

C'est le prix du widget injecté, choisi contre un vrai gain : `auth` garde ses
règles de validation et ses messages d'erreur chez lui, et le jour où il ajoute
une vérification, `spaces` ne bouge pas. Le seul filet est la carte 384.

**Trois violations trouvées sans les chercher** : le doublon de type
d'événement, le chargement amputé, et la requête de `spaces` sur `auth__users`.
Les trois dormaient dans du code qui compilait et dont les tests passaient.

## Terminé quand

Sur la démo, un administrateur d'espace ouvre `/app/{space_id}/admin`, y voit
ses membres avec leur rôle, **promeut puis rétrograde** un coach, **en retire**
un, et **fait entrer deux nouveaux venus** — l'un depuis l'annuaire, l'autre par
création de compte. Le second **reçoit son email de définition de mot de passe
et parvient à se connecter**. Un `SpaceUser` qui ouvre la même URL reçoit un
403, et la page ne produit **aucun décalage** au chargement.

Le critère enchaîne délibérément la création de compte jusqu'à la connexion :
c'est le seul bout par lequel on vérifie que le contrat `accountCreated` tient
et que l'email part vraiment.
