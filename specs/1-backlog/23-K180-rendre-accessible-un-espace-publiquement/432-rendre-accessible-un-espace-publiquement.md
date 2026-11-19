# Rendre un espace accessible publiquement

**Épic :** aucune · **État :** contenu à définir

## Ce qu'on sait

**Le titre, et son histoire de numérotation.** La carte a d'abord porté le
numéro **61**, déjà pris par `61-players-bc-structure-aggregate` ; elle a été
renumérotée en 432. Son corps n'a jamais été écrit — le fichier est resté vide
plusieurs mois, ce qui ne se voyait pas.

## Ce qu'il faut trancher avant d'écrire quoi que ce soit

Un espace est aujourd'hui **entièrement privé** : `space_scope` refuse en `404`
toute ressource d'un espace dont on n'est pas membre. « Publiquement accessible »
peut vouloir dire trois choses très différentes :

- **en lecture seule pour tout visiteur**, connecté ou non — classements,
  équipes, résultats, sans les écrans d'administration ;
- **en lecture pour tout coach connecté**, mais pas pour un anonyme ;
- **une vitrine choisie** — l'espace publie ce qu'il veut, page par page.

La première ouvre une question de fond : `space_scope` est le verrou qui a
fermé treize routes d'administration (carte 416), et son principe est qu'une
ressource d'un autre espace **n'existe pas**. Une exception de lecture publique
doit être un chemin distinct, pas un assouplissement de ce verrou.

**Tant que la portée n'est pas tranchée, il n'y a pas de carte** — il y a un
titre.
