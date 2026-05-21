### Contexte compétition - Modèle de persistance 

## données externes
- utilisation de cache pour les données externes : 
  - user
  - space
  - association space / coach / coach profile

## données de domaine
- competitions : 
    - DraftCompetition: modèle tabulaire en lecture et écriture
    - StructuredCompetition: event_sourcées en écriture, et utilisation de projection en lecture
    - RulesetChosenCompetition: event_sourcées en écriture, et utilisation de projection en lecture
    - ReadyCompetition: event_sourcées en écriture, et utilisation de projection en lecture
