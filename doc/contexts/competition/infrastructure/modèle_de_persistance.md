### Contexte compétition - Modèle de persistance 

## données externes
- utilisation de cache pour les données externes : 
  - user
  - space
  - association space / coach / coach profile

## données de domaine
- competitions : 
    - DraftCompetition: event_sourcées en écriture, et utilisation de projection en lecture
    - StructuredCompetition: event_sourcées en écriture, et utilisation de projection en lecture
    - RulesetChosenCompetition: event_sourcées en écriture, et utilisation de projection en lecture
    - Ready
