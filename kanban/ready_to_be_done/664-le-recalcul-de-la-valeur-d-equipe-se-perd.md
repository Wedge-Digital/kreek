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

Aucune migration de schéma ; les événements existants ne changent pas.

### Ce qui existe aujourd'hui

- **Aucune sous-commande** de `src/cli/` ne recalcule une valeur d'équipe :
  `serve`, `seed-accounts`, `seed-e2e`, `send-notifications`, rien d'autre.
- **Un recalcul de toutes les équipes existe déjà**, sous forme de migration de
  données : `infrastructure/data_migrations/m002_recalcul_valeurs_equipe.rs`
  (carte 388). Elle boucle sur `team_proj` et appelle
  `recompute_team_value_use_case::execute` — le mécanisme nominal, désormais
  muni des nouvelles tentatives. Elle est marquée appliquée en production
  depuis la carte 388 : elle ne repassera pas.

### Le piège : `docker logs` ne remonte pas au-delà du conteneur courant

Un redéploiement **recrée** le conteneur, et le journal de l'ancien part avec
lui. Chercher après le déploiement ne trouve donc rien d'antérieur ; chercher
avant ne trouve que ce qui a eu lieu depuis le dernier déploiement. La liste des
équipes touchées n'est **jamais complète** — c'est ce qui fait préférer un
recalcul de toutes les équipes à un recalcul ciblé.

La recherche, à lancer **avant** de déployer, sur le conteneur encore en place
(`<conteneur>` : le nom du conteneur applicatif) :

```bash
docker logs <conteneur> 2>&1 \
  | grep 'team_value_listener' | grep 'écriture concurrente détectée' \
  | grep -oE 'recalcul de [0-9A-Z]{26}' | sort | uniq -c
```

Elle rend les équipes qui ont perdu un recalcul depuis le dernier démarrage, et
combien de fois. Après le correctif, la même recherche ne doit plus rien rendre :
le libellé de l'épuisement (`… (3 tentatives épuisées)`) garde la même chaîne, à
dessein, et une ligne qui sortirait encore serait un vrai défaut. Les nouvelles
tentatives, elles, se journalisent en `warn` sous un autre libellé — `conflit de
version, relecture et nouvelle tentative` — qui ne pollue pas cette recherche :

```bash
docker logs <conteneur> 2>&1 | grep 'conflit de version, relecture'
```

### Proposition — à valider avant d'écrire

**A. Une migration de données `m008`, recommandée.** Une copie de `m002`, sans
sa garde `LOW_COST_LINEMEN` (la règle est au corpus de production depuis la
carte 388), nommée `664-recalcul-valeurs-equipe` et ajoutée en fin de
`registre()`. Elle recalcule **toutes** les équipes une fois, au premier
démarrage du binaire déployé, avant que le serveur n'écoute. Aucune commande à
lancer : l'utilisateur constate seulement qu'elle est passée.

```bash
docker logs <conteneur> 2>&1 | grep '664-recalcul-valeurs-equipe'
# attendu : « migration de données appliquée … agregats=<nombre d'équipes> »
```

Coût : un `TeamValueRecomputed` de plus dans l'historique de chaque équipe, y
compris celles dont la valeur était juste — `m002` en a fait autant. Risque : un
échec refuse le démarrage ; avec les nouvelles tentatives, seule une panne de
base le provoquerait, et elle le provoquerait de toute façon.

**B. Une sous-commande `kreek recompute-team-values [--team <id>]…`**, si l'on
veut un outil réutilisable plutôt qu'un rattrapage unique. Le plus petit
nécessaire :

- un variant `RecomputeTeamValues { #[arg(long = "team")] teams: Vec<String> }`
  dans `Command` (`main.rs`), et sa branche dans `match`, qui sort en `1` au
  premier échec, comme `send-notifications` ;
- `src/cli/recompute_team_values.rs` : `compose(cfg, pool)` pour obtenir
  `state.teams`, la liste donnée — ou toutes les équipes de `team_proj` si
  elle est vide —, puis `recompute_team_value_use_case::execute` équipe par
  équipe, avec une ligne `info` par équipe et un total ;
- un test `#[sqlx::test]` sur une équipe seedée.

La commande, après déploiement :

```bash
cd /srv/kreek && EXEC_PROFILE=prod ./kreek recompute-team-values
```

(ou, si l'application tourne en conteneur, la même sous-commande par
`docker exec <conteneur> kreek recompute-team-values`.)

B demande un geste manuel qu'on peut oublier ; A non. Comme la recherche
`docker logs` ne peut de toute façon pas lister toutes les équipes touchées,
le ciblage `--team` de B n'apporte rien ici. **A est proposée** ; B reste
l'option si l'on veut garder l'outil.
