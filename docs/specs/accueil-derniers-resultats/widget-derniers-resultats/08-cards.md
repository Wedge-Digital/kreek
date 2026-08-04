# Widget "Derniers résultats" — Phase 8 : Cartes kanban

6 cartes créées dans `kanban/ready_to_be_done/`, ordonnées par dépendance.
(Une 7e carte, 277, a été ouverte séparément pour une violation architecturale
découverte en Phase 3, hors scope de cette feature.)

| # | Carte | Dépend de | Résumé |
|---|---|---|---|
| 278 | `competitions-latest-results-projection` | — | Migration `published_at` + mise à jour du listener existant |
| 279 | `competitions-latest-results-repository` | 278 | DTO + requête jointe + méthode repository |
| 280 | `competitions-latest-results-authorization` | 279 | Autorisation par ligne + construction des VMs |
| 281 | `competitions-latest-results-widget` | 279, 280 | Handler + template + route du widget |
| 282 | `news-home-latest-results-integration` | 281 | Remplacement du bloc statique sur l'accueil |
| 283 | `accueil-latest-results-e2e` | 282 | Tests E2E |

Chaîne strictement séquentielle : chaque carte dépend directement de la
précédente (projection → lecture → autorisation → widget → intégration →
E2E), pas de parallélisation possible ici.
