

# Contexte compétition :

## Objectif :
Prendre en charge la gestion des compétitions dans l'espace, y compris la création, la modification, la démarrage et l'archivage des compétitions, ainsi que la gestion des utilisateurs et des administrateurs.

## Commandes: 
- Créer une compétition brouillon
- Changer les règles d’une compétition
- Changer la structure d’une compétition
- Créer une saison d’une compétition
- Démarrer une saison
- Archiver une saison
- Archiver une compétition

## Querys: 
- la liste des utilisateurs par espace
- la liste des administrateurs de l’espace
- la liste des compétitions de l’espace
- les détails d’une compétitions :

## App Events consommés: 
- Auth : user enregistré
- Auth : user banni
- Space : espace créé
- Space : user promu
- Space : espace archivé

## App Events produits:
- compétitions créé
- compétition prête pour inscription
- compétition lancée
- compétition archivée

## données externes cachées:
- la liste des utilisateurs 
- la liste des espace 
- les liens entre les utilisateurs et les espace
