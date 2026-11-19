# L'axe 8 de `check-arch` ne vérifie rien

**Priorité : haute** — un verrou bloquant qui affiche vert sans rien lire
**Dépend de :** rien · **Sans épic**
**Trouvée par :** la carte 437, en y inscrivant un test e2e neuf

## Le constat

`make check-arch` affiche :

```
Axe 8 · Carte d'impact e2e — exhaustive et sans entrée morte
  ✓ PASS
```

Il n'a jamais lu `impact-map.toml`. L'axe importe `tomllib`, entré dans la
bibliothèque standard en **Python 3.11** ; le `python3` du système est en
**3.9.6**. Trois choses conspirent :

```bash
axe8=$(python3 - <<'PY' 2>/dev/null || true   # ① stderr jeté  ② code de sortie ignoré
…
PY
)
if [ "$count8" -gt 0 ]; then print_fail "$axe8"; else print_pass; fi   # ③ le verdict est la sortie
```

Un `ModuleNotFoundError` n'écrit rien sur `stdout`. **« Aucune anomalie » et « le
programme n'a pas démarré » deviennent indistinguables**, et le second se lit
comme un succès.

C'est mot pour mot ce que le `CLAUDE.md` reproche à l'ancien `make lint` — *une
étape sautée doit échouer, pas rassurer*. Pire qu'une cible non branchée, qui au
moins ne prétend rien.

## Ce que ça coûte déjà

Rejoué avec un interpréteur moderne, l'axe remonte **cinq tests e2e sans entrée
dans `impact-map.toml`**, ajoutés entre le 25 et le 29 août :

```
test_lineman_a_vil_prix        test_space_admin_ajout_direct
test_noms_typographiques       test_space_admin_membres
test_saison_non_finalisee
```

Un test absent de la carte est traité comme transverse par le skill
`test-impact` : toujours exécuté, donc la sélection locale ne sélectionne plus
rien. Le préjudice est modeste — la carte n'a d'autorité qu'en local — mais il
est réel, et surtout **il ne se serait jamais vu**.

## Ce qui a permis à personne de le voir

`make check-arch` tourne à deux endroits, et aucun ne le voyait.

**En local**, l'axe ne démarre pas, faute de `tomllib`.

**En CI**, il démarrerait — `ubuntu-latest` a un Python moderne — mais la CI ne
se déclenche que sur `main` et sur les pull requests vers `main`. Tout le
travail vit sur `demo`. Le workflow n'a donc **jamais vu ces cinq fichiers**.

## La correction

### 1. Le verdict passe par le code de sortie

C'est la correction de fond ; les deux autres ne feraient que déplacer le
problème. Le patron correct est **déjà dans le dépôt** :
`scripts/check-css-collisions.sh` termine par `sys.exit(1 if … else 0)`, lit
`CODE=$?`, et un plantage y devient un échec.

L'axe 8 doit faire pareil. `2>/dev/null` disparaît : quand un contrôle casse, on
veut lire pourquoi.

**Les autres axes gardent leur `|| true`, et c'est correct** : ce sont des
`grep`, qui sortent en 1 quand ils ne trouvent rien. Le `|| true` y traduit
« aucune violation », pas « le programme n'a pas tourné ». La confusion n'existe
que pour un axe dont la commande est un programme.

### 2. Le lecteur ne dépend plus de `tomllib`, et devient partagé

Un lecteur strict d'une vingtaine de lignes, du sous-ensemble que le fichier
emploie : deux tables, des clefs entre guillemets, des tableaux de chaînes
éventuellement sur plusieurs lignes, des commentaires `#`.

Il vit dans `scripts/impact/lire_carte.py` et **sert les deux consommateurs de
la carte** : l'axe 8 et `scripts/impact/select_tests.py`. Deux analyseurs qui
divergeraient seraient pires qu'un seul imparfait — et le second avait
exactement la même panne (voir ci-dessous).

**Strict veut dire : toute ligne incomprise lève une erreur, jamais un
silence.** C'est ce qui le rend sûr — et c'est la même exigence que celle qu'on
vient de poser sur l'axe lui-même.

Vérifié avant d'être proposé : sur le fichier réel, il rend un résultat
**identique à `tomllib`** — 58 entrées de tests, 11 de dépendances.

L'autre voie — exiger Python ≥ 3.11 et échouer sinon — serait honnête, mais
laisserait `make check-arch` rouge sur les machines dont le `python3` est plus
ancien, pour un remède qui n'est pas dans le dépôt. Elle est écartée, pas
oubliée.

### 3. Les cinq entrées manquantes

| Test | BCs traversés |
|---|---|
| `test_lineman_a_vil_prix` | competitions, teams, players, references, team_creation, ranking, spaces |
| `test_noms_typographiques` | competitions, teams, players, references, team_creation, ranking, spaces |
| `test_saison_non_finalisee` | competitions, team_creation, teams, references, ranking, spaces |
| `test_space_admin_ajout_direct` | spaces, auth |
| `test_space_admin_membres` | spaces, auth |

### 4. `make test-impacted` échoue quand son sélecteur ne tourne pas

Le même `import tomllib` tuait `scripts/impact/select_tests.py`, le sélecteur
local de tests e2e. Il affichait bien sa trace — `stderr` n'y était pas étouffé
— mais son code 1 tombait dans le `elif [ -n "$tests" ]` final : sélection
vide, aucun test exécuté, **et la cible rendait zéro**.

Un code non nul autre que 10 ou 11 fait désormais échouer la cible. C'est la
même règle que pour l'axe 8, appliquée là où elle manquait aussi.

Trouvé en corrigeant l'axe : la panne était la même, dans le fichier voisin.

### 5. La CI se déclenche aussi sur `demo`

Sans quoi la correction ne serait vérifiée que par la machine qui l'a écrite.
`demo` est la branche de travail : c'est là que les tests naissent, donc là que
l'axe doit les voir.

Les quatre jobs y tournent tels quels — ils sont autonomes (service Postgres,
`.env.dev` généré, seed synthétique, serveur lancé) et ne dépendent d'aucun
secret ni d'aucun état de `main`.

Le coût est assumé : le pipeline complet à chaque poussée sur `demo`, `e2e`
compris. C'est le prix d'un verrou qui garde vraiment — et les quatre jobs
tournent déjà à chaque pull request.

## Ce que la carte ne fait pas

**L'axe 7.** Même forme — `find | xargs awk … 2>/dev/null || true` — donc un
plantage d'`awk` s'y lirait aussi comme un succès. Il reste en l'état pour deux
raisons : il est **non bloquant** (il compte des avertissements), et l'absence
d'`awk` n'est pas un scénario réel comme l'est celle d'un module de
bibliothèque standard entré en 3.11. Signalé ici pour ne pas être redécouvert.

**Les autres consommateurs de `python3`.** `check-css-collisions.sh` est déjà
correct — c'est lui qui sert de modèle.

## Tests

Le verrou se falsifie, sinon on referait la faute qu'on corrige :

- casser volontairement le lecteur → l'axe **rougit** au lieu de passer ;
- retirer une entrée d'`impact-map.toml` → l'axe la signale ;
- ajouter un fichier `test_*.py` sans entrée → l'axe le signale ;
- écrire une ligne hors du sous-ensemble → l'axe s'arrête dessus, il ne la
  saute pas ;
- casser le lecteur partagé → `make test-impacted` échoue au lieu de rendre
  zéro sans rien exécuter.

## Checklist

- [x] Verdict par code de sortie, `2>/dev/null` retiré
- [x] Lecteur sans `tomllib`, strict, partagé, vérifié identique à `tomllib`
- [x] `select_tests.py` passe au lecteur partagé
- [x] `make test-impacted` échoue si son sélecteur ne tourne pas
- [x] Les cinq entrées
- [x] `demo` dans les déclencheurs de la CI
- [x] Les falsifications
- [x] `make check-arch` passe, et pour de bon

---

# Ce que la réalisation a appris

## Le même module manquant tuait aussi la sélection de tests

L'axe 8 était le symptôme visible. `scripts/impact/select_tests.py` importait le
même `tomllib`, et `make test-impacted` **n'exécutait plus aucun test tout en
rendant zéro** : la trace s'affichait, le code 1 tombait dans un `elif` qui ne
le traitait pas, et la cible passait.

C'est pourquoi le lecteur est **extrait plutôt que recopié**. Deux analyseurs
d'un même fichier finissent par diverger, et la carte n'a qu'une vérité.

## Falsification

| Mutation | Constaté |
|---|---|
| Un import manquant dans l'axe 8 | `✗ FAIL` avec la trace — c'est le défaut d'origine, remis |
| Une entrée retirée de la carte | `✗ FAIL`, le test manquant nommé |
| Un `test_*.py` sans entrée | `✗ FAIL`, le fichier nommé |
| Une ligne hors du sous-ensemble (`seuil = 3`) | `✗ FAIL`, ligne et contenu nommés |
| Le lecteur partagé cassé | `make test-impacted` sort en erreur |

Avant cette carte, **les cinq passaient au vert.**

## Ce qui reste, et pourquoi

**L'axe 7** garde son `find | xargs awk … 2>/dev/null || true`. Il est non
bloquant, et l'absence d'`awk` n'est pas un scénario réel comme l'est celle d'un
module entré dans la bibliothèque standard en 3.11. Signalé pour ne pas être
redécouvert.

**Les autres axes gardent leur `|| true`**, et c'est correct : ce sont des
`grep`, qui sortent en 1 quand ils ne trouvent rien. Le `|| true` y traduit
« aucune violation », pas « le programme n'a pas tourné ». La confusion n'existe
que pour un axe dont la commande est un programme — il n'y en avait qu'un.
