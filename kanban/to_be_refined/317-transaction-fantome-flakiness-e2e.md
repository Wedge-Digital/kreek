# Transactions fantômes — la vraie cause de la flakiness e2e

**Priorité : haute** — elle fait échouer la CI au hasard
**Contexte :** transverse — pool de connexions, cycle de vie des transactions

## Observé, pas supposé

Pendant la carte 309, sur le serveur de développement :

```
PID 29726  idle in transaction   ouverte depuis 3 min 18   wait_event: ClientRead
           dernière requête : INSERT INTO team_roster_selections …
           → bloquait 3 connexions

PID 29209  active  bloqué depuis 2 min 35  UPDATE competition_seasons …
PID 29244  active  bloqué depuis 2 min 22  INSERT INTO team_roster_selections …
PID 29174  active  bloqué depuis 1 min 26  INSERT INTO team_roster_selections …
```

`pg_terminate_backend(29726)` a tout débloqué instantanément.

## Ce que ça explique

Deux exécutions de la suite complète, à quelques minutes d'écart :

| Run | Échecs |
|---|---|
| avec `test_player_customisation` | `recruitment_phase` (10), `spp_scale` (3), `special_rule_selector` (3), `ranking_tiebreak` (1) |
| sans | `competition_full_lifecycle`, `phase2_pickers` |

**Aucun recoupement.** Et chacun de ces tests **passe relancé seul**. Tous les
échecs sont des `Timeout 30000ms` sur `page.goto` ou `APIRequestContext.post`.

Un vrai défaut tombe au même endroit. Celui-ci se déplace : il frappe le test
qui a la malchance de passer pendant qu'une transaction fantôme tient un verrou.

## Diagnostic

### Ce qui est écarté

**`TeamRosterRepository::save` n'ouvre aucune transaction** — `execute(&self.pool)`
nu. Piège de lecture : `pg_stat_activity.query` montre la **dernière requête
exécutée sur la connexion**, pas celle qui a ouvert la transaction. L'`INSERT`
observé est un passager, pas le coupable.

**Aucun `commit` oublié.** Les treize sites `.begin()` du projet committent tous
en moins de 55 lignes, sans `await` sur un port ou un bus au milieu.

### L'hypothèse que les faits soutiennent

**L'annulation de requête.** Quand un client abandonne — Playwright au bout de
30 s — axum *drop* le future du handler. Si cela tombe entre le `BEGIN` et le
`COMMIT`, le `Drop` de `sqlx::Transaction` ne peut pas `await` son `ROLLBACK` :
il le met en file, et la connexion retourne au pool **encore dans sa
transaction**.

`wait_event: ClientRead` est la signature : Postgres attend un client qui ne
parlera plus.

La connexion empoisonnée est ensuite réattribuée à un handler suivant, dont la
requête s'exécute dans la transaction fantôme et en devient la dernière trace
visible.

**C'est un cercle vicieux** : une requête lente provoque un timeout, le timeout
fuit une transaction, la transaction bloque les suivantes, qui timeoutent à leur
tour.

### Ce qui l'aggrave

```
config/dev.toml : max_connections = 20, idle_timeout = 600 s
.env.dev        : DATABASE__MAX_CONNECTIONS=100   (surcharge)
ni max_lifetime, ni test_before_acquire
```

`idle_timeout` ne recycle que les connexions oisives **saines**. Une connexion
`idle in transaction` peut donc rester dix minutes à bloquer les autres — ce qui
a été observé.

## Réalisé — leviers 1 et 2, et le garde-fou

`init_pool` pose `SET idle_in_transaction_session_timeout = '15s'` via
`after_connect`, plus `max_lifetime` à 30 min. Le réglage vit dans le dépôt et
vaut pour dev, test et production, au lieu d'un `ALTER ROLE` qui n'existerait
que sur une base.

`test_before_acquire` avait été ajouté puis **retiré** : c'est déjà le défaut de
sqlx 0.8, et son ping réussit parfaitement sur une connexion oisive dans une
transaction — il détecte une connexion morte, pas empoisonnée. Le laisser aurait
donné l'illusion d'un second garde-fou.

`conftest.py` gagne `_fail_on_leaked_transactions`, bloquant : il échoue sur le
test **qui a fui**, pas sur celui qui en paie le prix. C'est ce décalage qui
rendait la flakiness illisible.

### Mesures

| Run | Résultat |
|---|---|
| avant, avec le nouveau fichier | 4 échecs, 14 erreurs |
| avant, sans | 2 échecs |
| après, run 1 | 1 échec, 10 erreurs |
| après, run 2 | **182 passés, 0 échec** |
| après, run 3 | **182 passés, 0 échec** |

Deux runs verts consécutifs après deux runs rouges — mais le run 1 d'après
correctif était encore rouge. **Le correctif améliore nettement sans être
prouvé suffisant.**

Détail qui compte : l'échec du run 1 d'après correctif n'était **pas** un
timeout mais une assertion — « Vous devez recruter au moins 11 joueurs ». Autre
mode de panne, autre cause : de l'état accumulé dans l'espace partagé, ce qui
relève de la carte 312.

## Ce qui reste — le levier 3

Le handler qui fuit n'est **pas identifié**. Le garde-fou n'a fired sur aucun
run vert, ce qui est cohérent : le timeout à 15 s tue les fuites, mais elles se
produisent peut-être encore sous le seuil de 5 s du garde-fou.

Reste donc à faire, si la flakiness revient : un `tracing` au `BEGIN` et au
`COMMIT` portant la route, pour nommer le coupable au lieu de le contenir.

## Écartés, et pourquoi

`statement_timeout` et `transaction_timeout` (ce dernier nouveau en PG 17)
frappent aussi les transactions **actives** : une migration longue au démarrage
en mourrait. `idle_in_transaction_session_timeout` ne touche que l'oisiveté
*dans* une transaction — exactement le mode de panne, et rien d'autre.

## Ce que cette carte n'est pas

Elle ne remplace pas la **carte 312** (temps d'exécution de la suite). Celle-ci
optimise des fixtures ; celle-là répare une fuite. Mais elle la **précède en
priorité** : 312 attribuait la flakiness à la charge, et la charge n'en est que
le déclencheur.

## Reproduction : rendue inutile pour l'instant

Abandonner une requête en vol pour observer le pool aurait écrit dans la base de
développement. Le garde-fou de `conftest.py` rend le geste superflu : il suffit
de relancer la suite pour que la fuite se nomme elle-même, si elle revient.
