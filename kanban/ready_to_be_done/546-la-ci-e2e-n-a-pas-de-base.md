# La CI e2e n'a pas de base, et ne le dit pas

**Priorité : haute — le job « Tests E2E » est rouge depuis six jours**
**Épic :** aucune — dette d'intégration continue
**Fichiers :** `.github/workflows/ci.yml`, `scripts/e2e_db.sh`

## Le symptôme

```
make: *** [Makefile:136: e2e_db] Error 2
Error: Process completed with exit code 2.
```

Rien d'autre. Pas de message, pas de piste.

## La cause

`make e2e` dépend de `e2e_db`, qui lance `scripts/e2e_db.sh`. Ce script lit
**`.env.e2e`** — et la CI ne le génère jamais : elle ne fabrique que `.env.dev`.

```
$ grep -c "env.e2e" .github/workflows/ci.yml
0
```

Reproduit à l'identique en local :

```
$ PROFIL=inexistant ./scripts/e2e_db.sh; echo $?
2
```

## Pourquoi le script se tait alors qu'il a un message pour ça

Il porte un garde-fou écrit pour exactement ce cas :

```bash
if [ -z "$URL" ] || [ -z "$BASE" ]; then
  echo "e2e_db: .env.$PROFIL ne définit pas DATABASE__URL et DATABASE__NAME" >&2
```

**Il est inatteignable.** Sous `set -euo pipefail`, `grep` sur un fichier absent
sort en **2**, `pipefail` propage ce 2 à la substitution de commande, et `set -e`
tue le script à la **première** lecture — vingt lignes avant le message.

C'est la forme la plus coûteuse du défaut : un garde-fou qui existe, qu'on croit
en place, et qui ne se déclenche jamais dans le seul cas qu'il vise.

## Depuis quand

Commit `0134c07`, carte 536, le 2026-09-08 — celle qui a donné sa propre base à
la suite. La CI n'a pas suivi, et `ci.yml` n'a pas été touché une seule fois dans
les **35 commits** suivants. Ce n'est la régression d'aucune carte récente.

## Le piège qu'une correction naïve ouvrirait

Générer un `.env.e2e` et s'arrêter là ferait passer `e2e_db` — et laisserait le
serveur tourner sur `kreek_db` (via `.env.dev`) pendant que `db_helpers.py`
interroge `kreek_e2e`. La suite assertirait alors **contre une base que le serveur
n'écrit pas** : verte sans rien mesurer, ou rouge sans motif lisible.

Les deux doivent donc porter la même base.

## Ce qui a été fait

**`.github/workflows/ci.yml`** — quatre changements au job `e2e` :

1. un second fichier d'environnement, `.env.e2e`, sur `kreek_e2e` ;
2. `./scripts/e2e_db.sh` **avant** le serveur — la base doit exister au
   démarrage ; le script construit le gabarit (migrations + `seed-e2e`) puis clone ;
3. le serveur lancé en `EXEC_PROFILE=e2e`, le mécanisme même de `make dev-e2e`,
   pour que la CI exerce le montage qu'on utilise ;
4. `make reset_db && make seed_e2e` retirés : ils préparaient `kreek_db`, que plus
   personne ne lit dans ce job.

`.env.dev` reste généré. Il ne sert plus au serveur, mais rien ne garantit
qu'aucun script du job n'y retombe — et si l'un le fait, il échouera bruyamment
sur une base inexistante plutôt que d'écrire en silence au mauvais endroit.

**`scripts/e2e_db.sh`** — le garde-fou devient atteignable : `|| true` en queue du
tuyau de lecture, et un contrôle d'existence du fichier **avant** toute lecture,
qui dit ce qui manque et où le produire.

## Ce qui n'est pas vérifiable d'ici

Le script est éprouvé dans les deux sens en local — message et code 1 sur profil
absent, clonage nominal sur `.env.e2e`. **Le câblage YAML, lui, ne se vérifie
qu'en poussant** : sa syntaxe est validée (`yaml.safe_load`) et l'ordre des
quatorze étapes relu, rien de plus. Le premier passage de CI est la vérification.

## Checklist

- [x] `.env.e2e` généré par le job, sur la même base que le serveur
- [x] `e2e_db.sh` lancé avant le serveur
- [x] `EXEC_PROFILE=e2e` au démarrage du serveur
- [x] `reset_db`/`seed_e2e` retirés du job e2e — vérifié qu'ils n'y servent plus
- [x] Le garde-fou du script atteignable, et son message éprouvé
- [x] `make e2e_db` passe toujours en local
- [ ] **Le job « Tests E2E » repasse au vert** — se constate au prochain passage
