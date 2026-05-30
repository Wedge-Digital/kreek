

# Contexte team creation :

## Objectif :
Objectif prendre en charge toute la logique liée à la création d'une équipe. La création d'une équipe requiet un context à 
part entière, car elle nécéssite des données externes, dont nous n'aurons plus l'utilité ensuite, pas la peine donc 
de les stocker.


## Commandes: 
- créer une équipe Brouillon:


- choisir un roster
- recruter un joueur
- recruter un inducement
- recruter du staff
- 

### Détails des commandes:

#### CreateDraftTeamCommand:



###### Structure de la commande:
la commande de création d'équipe brouillon, contient : 
- **un nom:** TeamName, Obligatoire
- **un logo:**, CloudinaryUrl, Obligatoire
- **un coach:**, TeamCreationCoach
- **une compétition:**, TeamCreationCompetition, Obligatoire

###### detail des types: 
- TeamCreationCompetition :
  - CompetitionId,
  - CréationRules (liste des tiers et leurs attribution)
- TeamName, même règle de formattage que SpaceName
- TeamCreationCoach: 
  - CoachId, ULID
  - Nom, CoachName,

###### action de domaine déclenchée:
- création simple d'un objet TeamDraft.

###### Invariant de domaine : 
- la competition doit être dans un état compatible de l'utilisation pour créer une équipe, cad complétement crée.
- 
