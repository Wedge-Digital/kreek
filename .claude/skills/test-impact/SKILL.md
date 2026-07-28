---
name: test-impact
description: >
  Sélectionne et exécute uniquement les tests e2e impactés par les modifications
  en cours, à partir de la carte d'impact tests↔bounded-contexts. À utiliser
  avant tout lancement de tests e2e en local, quand l'utilisateur demande de
  "lancer les tests impactés", après une série de modifications, ou quand un
  nouveau test e2e est ajouté (mise à jour de la carte obligatoire). Ne remplace
  jamais la suite complète en CI.
---

# Test Impact Analysis — sélection des tests e2e

## Principe

La suite e2e complète prend ~7 min 30 pour 29 tests, l'essentiel du temps étant
la construction des compétitions de fixture par l'UI. Ce skill sélectionne le
sous-ensemble potentiellement impacté par les modifications courantes, en trois
étapes :

1. **Résolution mécanique** : fichiers modifiés → bounded contexts touchés
   (délégué au script, jamais recalculé à la main).
2. **Amplification** : application des règles de propagation (ci-dessous).
3. **Sélection** : BCs impactés → tests à lancer, via `tests/impact-map.toml`.

Contrat de sécurité : ce skill est un **filtre local de productivité**.
La CI exécute TOUJOURS la suite complète. Ne jamais proposer de restreindre
la CI sur la base de cette analyse. En cas de doute à n'importe quelle étape :
élargir la sélection, jamais la réduire.

## Prérequis d'exécution

Les tests e2e ne démarrent pas le serveur (cf. `tests/e2e/README.md`) :

- serveur lancé en `make dev-demo` (jeu de démonstration + `BYPASS_AUTH=true`) ;
- base seedée : `make seed_e2e`.

Ne jamais démarrer ni redémarrer ce serveur soi-même (règle 8 du CLAUDE.md).
Vérifier qu'il répond, sinon demander à l'utilisateur de le lancer.

## Étape 1 — Résolution mécanique

```bash
scripts/impact/changed_bcs.sh [<ref>]
```

- Sans argument : working tree + index vs HEAD, **fichiers non suivis inclus**.
- Avec ref (ex. `main`) : ce que la branche a apporté depuis la divergence,
  plus le working tree.

Jetons émis, un par ligne :

| Jeton | Signification |
|---|---|
| `<bc>` | BC touché (module sous `src/app/`, ou son adapter sous `src/infrastructure/`) |
| `@contract:<bc>` | surface de contrat du BC (ports, événements, publisher, listeners) — émis **en plus** de `<bc>` |
| `@templates:<bc>` | template Askama du BC |
| `@shared_kernel` | `src/app/shared_kernel/`, `all_domain_events.rs`, `routes.rs` |
| `@core` | `main.rs`, `state.rs`, `src/web/`, `src/common/`, `src/cli/` |
| `@migrations` | migration SQL |
| `@config` | `config/`, `.env*`, `src/config.rs` |
| `@build` | `Cargo.toml`, lockfile, `Makefile`, `askama.toml`, CI |
| `@assets` | CSS, JS, templates de base, référentiel de règles |
| `@e2e_harness` | `conftest.py` et helpers partagés de la suite e2e |
| `@test:<nom>` | fichier de test e2e modifié |
| `@unknown:<path>` | fichier non résolu — trou de couverture du script |

Ne jamais deviner l'appartenance d'un fichier à un BC : si le script émet
`@unknown`, appliquer la règle d'amplification **et** signaler que le script a
un trou à corriger — dans le script, pas par interprétation.

## Étape 2 — Règles d'amplification

Appliquer dans l'ordre, s'arrêter à la première règle « run all » déclenchée :

| Déclencheur | Effet | Raison |
|---|---|---|
| `@shared_kernel` | **run all** | Types et événements partagés : impact potentiel sur tous les BCs |
| `@migrations` | **run all** | Le schéma est un contrat global |
| `@config` | **run all** | La config traverse les BCs |
| `@build` | **run all** | L'environnement d'exécution change |
| `@core` | **run all** | Plomberie transverse : layout, middleware, routeur, bus, seed |
| `@assets` | **run all** | CSS/JS globaux : un sélecteur masqué casse un test aussi sûrement qu'un bug serveur |
| `@e2e_harness` | **run all** | Toute la suite dépend de ces helpers |
| `@unknown:<path>` | **run all** + signalement | Trou de résolution = aucune garantie |
| `@contract:<bc>` | ajouter `<bc>` **+ ses dépendants `[deps]`** | Le contrat est ce que les autres BCs voient de lui |
| `@templates:<bc>` | ajouter `<bc>` | Les templates Askama sont compilés dans leur BC |
| `<bc>` | ajouter `<bc>` | — |
| `@test:<nom>` | sélectionner ce test, quoi qu'en dise la carte | Un test modifié se relance toujours |

### Pourquoi `[deps]` ne s'applique qu'au contrat

Les entrées de la carte déclarent **tout ce qu'un test traverse**, fixture
comprise. Le couplage inter-BC y est donc déjà présent : un test qui dépend de
`competitions` et de `match_report` liste les deux. Appliquer `[deps]` sur
n'importe quelle modification de BC compte ce couplage une seconde fois — le
graphe étant cyclique (`competitions ↔ match_report ↔ teams ↔ players`), la
sélection remonte à 29/29 pour 6 BCs sur 11, et le filtre ne filtre plus rien.

Le seul cas où les entrées ne suffisent pas est celui d'un changement invisible
depuis les tests du BC modifié mais cassant chez un voisin : un port, un
événement émis, un listener. D'où `@contract:<bc>`, qui est précisément la
détection mécanique de ce cas.

Effet mesuré sur la carte actuelle :

| BC modifié | code interne | contrat (`@contract`) |
|---|---|---|
| `spp_calculator` | 10/29 | 17/29 |
| `match_report` | 17/29 | 29/29 |
| `players` | 18/29 | 20/29 |
| `teams`, `team_creation` | 20/29 | 22-29/29 |
| `ranking`, `references` | 27/29 | 29/29 |
| `competitions`, `spaces` | 29/29 | 29/29 |
| `auth` | 0/29 → voir ci-dessous | 29/29 |
| `news` | 0/29 | 0/29 |

Un diff limité à `competitions`, `spaces` ou `references` sélectionne tout :
c'est la mesure du couplage réel de la suite, pas un défaut de la carte. Ne
jamais « corriger » ça en amputant des entrées.

## Étape 3 — Sélection et exécution

Lire `tests/impact-map.toml`. Un test est retenu si :

- il déclare `"all"`, **ou**
- l'intersection entre ses BCs déclarés et l'ensemble amplifié est non vide,
  **ou**
- son nom apparaît en `@test:<nom>`, **ou**
- il n'a **aucune entrée** dans la carte (traité comme `"all"` + signalement).

Exécution du sous-ensemble :

```bash
cd tests/e2e && uv run pytest test_a.py test_b.py -v
```

Exécution complète (dès qu'une règle « run all » se déclenche) :

```bash
make e2e
```

Puis produire un rapport court, toujours dans ce format :

```
Jetons          : competitions, @templates:competitions, @test:test_pairing_deletion
Amplification   : competitions (pas de @contract → pas de propagation [deps])
Tests exécutés  : 29/29 — …
Tests ignorés   : 0
Rappel          : la CI exécutera la suite complète.
```

Quand la sélection est vide, ne jamais l'afficher comme un succès : dire
explicitement quel BC n'a aucune couverture e2e.

## Trous de couverture connus (au 2026-07-28)

- **`news`** (articles, commentaires, fil d'accueil) : aucun test e2e. Un diff
  limité à ce BC ne sélectionne rien — le signaler, et proposer d'écrire le
  test manquant plutôt que de conclure « rien à lancer ».
- **`auth`** : aucun test e2e direct (login, inscription, mot de passe oublié).
  Le parcours réel est court-circuité par `BYPASS_AUTH` en local. Une
  modification de `auth` hors contrat ne sélectionne donc rien ; avec
  `@contract:auth`, elle sélectionne les tests de `spaces` via `[deps]`.
- Deux fichiers non suivis à la racine (`extract_history.sh`,
  `kreek-histoire-export.txt`) sortent en `@unknown` et forcent un run all
  tant qu'ils sont là. Ce n'est pas un bug du script : un fichier non classable
  à la racine ne peut pas être déclaré sans impact.

## Maintenance de la carte — obligations

La carte est une **déclaration à maintenir**, donc un point de mensonge
potentiel. Règles :

1. **Tout nouveau test e2e** doit être ajouté à `impact-map.toml` dans le même
   commit. Si l'utilisateur ajoute un test sans mise à jour, le signaler et
   proposer l'entrée — en **lisant le test** pour déterminer les BCs réellement
   traversés : routes appelées, fixtures utilisées, tables interrogées,
   événements attendus. Ne jamais deviner depuis le nom du fichier.
2. **Un test sans entrée** = traité comme `"all"` + signalement. Jamais ignoré
   silencieusement.
3. **Drift detection** : si un test échoue en CI alors qu'il n'avait pas été
   sélectionné en local pour le même diff, c'est un bug de la CARTE (BC manquant
   dans une entrée, ou dépendance manquante dans `[deps]`). Corriger la carte
   AVANT de corriger le code, et le dire explicitement.
4. **Ne jamais retirer** un BC d'une entrée pour accélérer les runs. Une entrée
   ne se réduit que si le test lui-même a changé de périmètre.
5. `[deps]` se dérive du code, pas de l'intention : listeners dans
   `src/app/*/io/app_events/`, imports des adapters dans
   `src/infrastructure/<consommateur>/`.

## Anti-patterns (refuser explicitement)

- Restreindre la CI à la sélection locale.
- « Le diff est petit, inutile de lancer quoi que ce soit. »
- Déclarer un test sur moins de BCs qu'il n'en traverse pour gagner du temps.
- Compenser un `@unknown` par une intuition au lieu de corriger le script.
- Présenter une sélection vide comme un succès alors qu'elle révèle un BC sans
  couverture e2e.
