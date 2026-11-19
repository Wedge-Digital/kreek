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
- `src/infrastructure/data_migrations/m008_recompute_team_values.rs` : le
  rattrapage des valeurs déjà fausses (ci-dessous).

## Tests

- **Unitaire** : un dépôt factice qui renvoie `ConcurrentWrite` puis accepte — la
  valeur est écrite ; un dépôt qui refuse toujours — l'erreur sort après N essais.
- **Migration de données** `m008` : une équipe dont la valeur est fausse en base
  est recalculée ; une seconde exécution n'écrit rien, la migration étant
  marquée appliquée.
- **E2E** : le test de `test_manual_phase_override.py` passe **20 fois de suite**.
- Une suite complète ne contient plus aucune ligne
  `team_value_listener.*écriture concurrente` dans le journal du serveur e2e.

## Le démarrage refusé par la migration (trouvé à la fusion, 2026-11-11)

Sur la base e2e (9 534 équipes), `m008` recalculait toutes les équipes puis
**échouait au marquage** : le serveur refusait de démarrer, et chaque essai
rejouait la migration entière. Cause : le pool pose
`idle_in_transaction_session_timeout = 15s` sur chaque connexion (carte 317) ;
la transaction de la migration reste oisive le temps de la boucle, qui passe par
le pool, et Postgres tuait la session au-delà de 15 s. En dev (274 équipes), la
boucle tenait sous ce délai.

Correctif dans le lanceur (`data_migrations/mod.rs`) : `SET LOCAL
idle_in_transaction_session_timeout = 0` en tête de la transaction de chaque
migration — la connexion retourne au pool avec le délai de la 317. Test
`une_migration_plus_longue_que_le_delai_d_oisivete_se_marque` : délai d'une
seconde, migration de deux ; il échouait sur « terminating connection due to
idle-in-transaction timeout » avant le correctif.

## Terminé quand

- [x] les tests unitaires passent (use case, migration `m008`, lanceur) ;
- [x] le test de `test_manual_phase_override.py` passe 10 fois de suite sur le
      vrai serveur ;
- [x] deux suites complètes d'affilée passent (544 tests, 2026-11-11). Le
      journal du serveur e2e n'a pas pu être relu pour y chercher
      `écriture concurrente` : il vit dans le terminal de l'utilisateur ;
- [ ] les valeurs périmées en production sont rattrapées (ci-dessous) — se
      constate au déploiement.

## Conséquences sur les données de production

**Oui.** Des valeurs d'équipe peuvent être fausses en production depuis un
recrutement, un renvoi ou un jet d'erreurs coûteuses dont le recalcul s'est perdu.
Le correctif du use case n'en répare aucune rétroactivement : c'est l'objet de la
migration de données `m008`.

Aucune migration de schéma ; les événements existants ne changent pas.

### Le rattrapage : la migration de données `m008`

`src/infrastructure/data_migrations/m008_recompute_team_values.rs`, nommée
`664-recalcul-valeurs-equipe`, en fin de `registre()`. C'est une copie de `m002`
(carte 388) sans sa garde `LOW_COST_LINEMEN`, qui ne protégeait que le premier
recalcul sous cette règle. Elle recalcule **toutes** les équipes de `team_proj`
par `recompute_team_value_use_case::execute` — le mécanisme nominal, avec ses
nouvelles tentatives —, **une seule fois**, au premier démarrage du binaire
déployé, avant que le serveur n'écoute. **Aucune commande à lancer.**

Toutes les équipes, et pas les seules touchées, parce que les touchées ne se
listent pas (ci-dessous). Recalculer une valeur juste la laisse juste.

**Coût** : un `TeamValueRecomputed` de plus dans l'historique de **chaque**
équipe, y compris celles dont la valeur était juste — `m002` en a fait autant.
Le démarrage est retardé du temps de ces recalculs, un par équipe, en série.

**Risque** : un échec refuse le démarrage (règle des migrations de données). Une
équipe que l'agrégat ne sait pas réhydrater suffirait. Aucune équipe sans
événement n'existe dans les bases de travail et e2e (vérifié le 2026-11-11) ; la
production n'a pas été interrogée.

### Avant le déploiement : mesurer l'étendue

`docker logs` ne remonte pas au-delà du conteneur courant : un redéploiement le
recrée, et le journal de l'ancien part avec lui. La recherche se lance donc
**avant** de déployer, sur le conteneur encore en place (`<conteneur>` : le nom
du conteneur applicatif). Elle ne sert plus à choisir les équipes à recalculer —
`m008` les prend toutes — mais à mesurer ce que le défaut a touché depuis le
dernier démarrage :

```bash
docker logs <conteneur> 2>&1 \
  | grep 'team_value_listener' | grep 'écriture concurrente détectée' \
  | grep -oE 'recalcul de [0-9A-Z]{26}' | sort | uniq -c
```

### Au premier démarrage : ce qu'on doit voir

```bash
docker logs <conteneur> 2>&1 | grep '664-recalcul-valeurs-equipe'
# attendu : « migration de données appliquée » avec agregats=<nombre d'équipes>
# aux démarrages suivants : « migration de données déjà appliquée »
```

### Ensuite : la recherche doit rester vide

Le libellé de l'épuisement (`… (3 tentatives épuisées)`) garde la chaîne
`écriture concurrente détectée`, à dessein : la recherche ci-dessus ne doit plus
rien rendre, et une ligne qui sortirait encore serait un vrai défaut. Les
nouvelles tentatives se journalisent en `warn` sous un autre libellé, qui ne
pollue pas cette recherche :

```bash
docker logs <conteneur> 2>&1 | grep 'conflit de version, relecture'
```

### Effet sur les autres bases

- **Base de travail** (`make dev`) et **base e2e** (`kreek_e2e`, persistante,
  remise à zéro seulement par `make init_e2e_db`) : la migration passe au
  prochain redémarrage de leur serveur sur ce code. Au 2026-11-11, la base e2e
  porte **~9 500 équipes** accumulées par la suite, la base de travail 274 : le
  serveur e2e n'écoute qu'une fois toutes recalculées, et un `make e2e` lancé
  pendant ce temps échouerait faute de serveur. Les équipes que la suite crée
  ensuite ne sont pas concernées. `make init_e2e_db` avant le redémarrage
  ramène ce coût à zéro.
- **CI** : base neuve, aucune équipe au démarrage — la migration se marque sans
  rien recalculer.
