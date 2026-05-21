
### SelectRuleSetForCompetition

## Objectif :
Initier la création d'une nouvelle compétition.

## Données attendues: 
- Nom de la compétition : maximum 100 caractères, espaces et caractères accentués autorisés, ainsi que caractéres spéciaux de ponctuation. 
- logo de la compétition: image cloudinary
- liste des administrateurs : liste d'identifiants d'utilisateurs
- id de l'espace: id de l'espace

## vérifications système:
- l'id de l'espace doit être valide et existant
- l'utilisateur connecté doit être administrateur de l'espace
- le nom de la compétition doit être unique par espace
- tous les identifiants passés dans la liste des administrateurs doivent être valides et existants

## logique de domaine: 
- aucune logique de domaine particulière n'est requise pour cette commande

## output: 
- emission d'un event de domaine CompetitionCreated, contenant les infos + l'id de competition CompetitionID