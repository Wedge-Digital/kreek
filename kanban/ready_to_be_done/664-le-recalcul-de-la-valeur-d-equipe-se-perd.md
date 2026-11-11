# Le recalcul de la valeur d'équipe se perd sur une écriture concurrente

**Priorité : haute** — une valeur d'équipe fausse en production, et un e2e instable
**Épic :** aucune · **Dépend de :** rien
**Trouvée par :** la CI du 2026-11-01 (`41353018956`) :
`test_manual_phase_override.py::test_un_recrutement_manuel_compte_la_recrue_dans_la_valeur_d_equipe`
attend 600 kPo et lit 550.

## Le constat

Deux app events déclenchent chacun un recalcul de la valeur d'équipe, et ils se
chevauchent. Journal CI : le recalcul de `ManualPhaseClosed` démarre à `.761`,
avant que la recrue soit écrite ; celui de `PlayersPlayerJoinedRoster` démarre à
`.764`, se heurte au premier sur la version de l'agrégat et **échoue** :
`ERROR team_value_listener [players] : Repository("écriture concurrente détectée")`.
Rien ne le refait ; la valeur reste à 550.

Le commentaire de `init_from_app_events` affirme qu'« un recalcul prématuré est
écrasé par celui-ci » : c'est faux quand les deux se chevauchent — c'est le second
à écrire qui perd. Le défaut est **systémique** : la CI du 2026-11-11 le montre
aussi, sur un renvoi qui chevauche un jet d'erreurs coûteuses.

## Ce qui change

- `src/app/teams/use_cases/recompute_team_value_use_case.rs` : sur
  `RepositoryError::ConcurrentWrite`, relire l'agrégat et recalculer, en nombre de
  tentatives **borné**, chaque nouvelle tentative journalisée. C'est la concurrence
  optimiste ordinaire : le recalcul est idempotent et part toujours de l'état relu.
- `src/app/teams/io/listeners/team_value_listener.rs` : le commentaire corrigé.
- L'épuisement des tentatives reste une erreur journalisée sous `kreek::`.

## Tests

- **Unitaire** : un dépôt factice qui renvoie `ConcurrentWrite` puis accepte — la
  valeur est écrite ; un dépôt qui refuse toujours — l'erreur sort après N essais.
- **E2E** : le test de `test_manual_phase_override.py` passe **20 fois de suite**.
- Une suite complète ne contient plus aucune ligne
  `team_value_listener.*écriture concurrente` dans le journal du serveur e2e.

## Terminé quand

- [ ] les tests ci-dessus passent ;
- [ ] la suite complète passe ;
- [ ] les valeurs périmées en production sont rattrapées (ci-dessous).

## Conséquences sur les données de production

**Oui.** Des valeurs d'équipe peuvent être fausses en production depuis un
recrutement, un renvoi ou un jet d'erreurs coûteuses dont le recalcul s'est perdu.
Le correctif n'en répare aucune rétroactivement.

Après le déploiement, **l'utilisateur** :
1. cherche le message `écriture concurrente détectée` émis par
   `team_value_listener` dans `docker logs` ;
2. recalcule une fois les équipes concernées — la carte précise la commande, à
   écrire avec le correctif (un recalcul existant, ou une sous-commande de
   `src/cli/` si aucun n'est déjà exposé).

Aucune migration ; les événements existants ne changent pas.
