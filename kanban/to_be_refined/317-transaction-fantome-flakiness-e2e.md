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

## Trois leviers, du plus sûr au plus profond

1. **Pool** — `max_lifetime` et `test_before_acquire`. Atténue : la connexion
   empoisonnée finit par être recyclée. Ne corrige pas la fuite.
2. **Postgres** — `idle_in_transaction_session_timeout` (et éventuellement
   `statement_timeout`) sur le rôle applicatif. Tue ces sessions
   automatiquement, sans toucher au code. C'est le filet, et il est solide.
3. **Trouver le coupable** — un `tracing` au `BEGIN` et au `COMMIT` portant la
   route, puis reproduire une annulation en vol. Donne le handler exact au lieu
   d'une hypothèse.

Les trois ne s'excluent pas. Le 2 devrait venir en premier : il transforme un
blocage de dix minutes en erreur immédiate et lisible, ce qui rend le 3
observable au lieu d'être noyé.

## Ce que cette carte n'est pas

Elle ne remplace pas la **carte 312** (temps d'exécution de la suite). Celle-ci
optimise des fixtures ; celle-là répare une fuite. Mais elle la **précède en
priorité** : 312 attribuait la flakiness à la charge, et la charge n'en est que
le déclencheur.

## Reproduction non tentée

Abandonner une requête d'écriture en vol et observer le pool écrirait dans la
base de développement. À faire avec l'accord de l'utilisateur, ou sur une base
jetable.
