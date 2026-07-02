# Onglets Résultats & Calendrier — Phase 6 : Domaine

## Pas de logique métier dans le BC competitions

Ces deux onglets sont en lecture pure. Aucune nouvelle méthode domaine, value object ou erreur domaine n'est nécessaire dans le BC `competitions`.

## Règles métier confirmées

| # | Règle | Origine |
|---|---|---|
| 1 | Un pairing sans rapport démarré reste `upcoming`, quelle que soit sa date | Phase 2 |
| 2 | Un pairing passe à `in_progress` dès qu'un rapport est démarré dans le BC `match_report` | Phase 3 |
| 3 | Un pairing passe à `completed` quand le rapport est finalisé dans le BC `match_report` | Phase 3 |
| 4 | Les données d'affichage (noms, logos) sont un snapshot au moment de la création du pairing | Phase 2 |
| 5 | 3 journées par page de scroll pour les deux onglets | Phase 2 |
| 6 | L'onglet Classement reste l'onglet par défaut | Phase 2 |

## Seul changement domaine : enrichissement de `PairingCreated`

L'event `PairingCreated` doit embarquer les données d'affichage au moment de sa création. Ce n'est pas une règle métier nouvelle — c'est une exigence de dénormalisation pour alimenter la projection sans port inter-BC.

Les données nécessaires sont disponibles au moment où `generate_pairings` / `generate_all_pairings` crée les pairings : le use case charge déjà les équipes et leurs métadonnées.

## Pas de tests unitaires domaine

Aucune règle métier du BC `competitions` n'est implémentée ici — les transitions de statut sont pilotées par le BC `match_report` via app events (voir Phase 7).
