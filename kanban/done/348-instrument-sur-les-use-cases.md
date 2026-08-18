# `app` — Chaque use case dit ce qu'on lui a demandé

**Priorité : haute** — c'est la carte qui répond au symptôme d'origine
**Dépend de :** carte 347 (`Debug` requis), carte 345 (sans `rid`, les lignes
ne se rattachent à rien)
**Fichiers :** les 71 fonctions instrumentées de `src/app/*/use_cases/`,
`src/common/services/observability/use_case_journal.rs`, `src/main.rs`,
`scripts/check-arch.sh`

## Le problème

Le comptage des appels à `tracing` dans `src/` :

| Niveau | Occurrences |
|---|---|
| `error!` | 198 |
| `warn!` | 74 |
| `info!` | 12 |
| `debug!` | 16 |

Sur les 12 `info!`, deux sont le middleware de requête, trois des commandes CLI,
sept des listeners. Autrement dit : **il n'existe aucune trace du chemin
nominal.** Le journal ne sait dire qu'une chose — « ça a cassé ». Quand rien ne
casse mais que le comportement est faux, il se tait ; les deux bugs du mode
customisation (cartes 326 et 327) en sont l'illustration.

Plus précisément : **sur 63 fichiers de `use_cases/`, un seul journalise quoi
que ce soit.** La couche qui sait *ce que l'utilisateur essayait de faire* est
muette, tandis que les 198 `error!` vivent dans les handlers et parlent de
sérialisation et de SQL. Le journal décrit la plomberie, jamais l'intention.

## Le piège : `#[instrument]` seul n'émet rien

C'est la découverte qui a changé la carte, et elle méritait de coûter une sonde
plutôt qu'une mise en production.

**`#[instrument]` crée un span ; il n'émet pas d'événement.** Un span ne devient
une ligne que si l'abonné est configuré avec `FmtSpan`, ce que `init_journal` ne
faisait pas. Posé seul sur des use cases muets — et 62 des 63 le sont —
l'attribut aurait enrichi les `error!` existants du contexte de la commande,
ce qui est utile, et **produit zéro ligne nouvelle**. Le chemin nominal serait
resté aussi inexistant qu'avant, avec l'apparence du travail fait.

Vérifié sur une sonde jetable :

```
--- use case muet :
                                              ← rien
--- use case qui journalise déjà :
ERROR execute{cmd=…}: kreek::…: une erreur comme il en existe 198
```

L'exemple de sortie que portait cette carte le trahissait d'ailleurs : sa ligne
`close time.busy=42ms` n'apparaît qu'avec `FmtSpan::CLOSE`, réglage que la
carte 345 avait **explicitement écarté** — « le réglage global émettrait une
paire de lignes par span, donc quatre par requête, et sa ligne de fermeture
porte le temps sans le statut ».

## Ce qu'il faut faire

### Une couche, pas un réglage global

`UseCaseJournal` (`common/services/observability/`) n'émet une ligne que pour
les spans dont la cible contient `::use_cases::`. Les spans de requête et
d'app event restent exactement comme la 345 les a laissés.

À la **fermeture** du span et non à l'ouverture : une ligne au lieu de deux, et
la durée y tient. Un panic ne fait pas de trou — le span est détruit pendant le
déroulement de pile, la ligne sort quand même. Le seul angle mort restant est
le use case qui *ne rend jamais la main* ; c'est le prix de la ligne unique, et
il est écrit dans le module.

La cible imprimée est celle de la couche, pas du use case : le chemin de module
part donc dans un champ `use_case=…`. Cette cible est `kreek::use_case` et non
le chemin réel du module — cinquante caractères sur chaque ligne sans rien
apprendre — mais elle **commence par `kreek::`**, sans quoi le filtre
`kreek=<niveau>,sqlx=warn` la rendrait muette. C'est la leçon de la carte 349,
qui avait perdu une demi-journée sur `tower_http::catch_panic`.

`init_journal` passe donc d'un `fmt().init()` à une composition de couches, la
couche ayant besoin d'un `registry` sous elle pour ranger l'état d'un span
entre son ouverture et sa fermeture.

### Les attributs

**58 fonctions à commande** portent `#[instrument(skip_all, fields(cmd = ?cmd))]`.
`skip_all` est indispensable : sans lui l'attribut tente d'enregistrer tous les
paramètres, dépôts compris, et `&dyn IPlayerRepository` n'implémente pas
`Debug`.

**13 use cases à paramètres scalaires** nomment leur champ un par un. La carte
n'en connaissait que trois ; tout `competitions/use_cases/admin/` fonctionne par
identifiants nus (`execute(season_id: &str)`), et ces dix-là auraient été
oubliés.

## Ce qu'on ne fait pas

**Les cinq fonctions de *services*** — `correction_eligibility_service`,
`customisation_basket_hydration_service`, `basket_hydration_service` (deux),
`team_value_service` — ne sont pas des use cases. Les instrumenter produirait
du bruit sans intention métier à raconter.

**Les trois lectures** — `dashboard_query`, `load_enrolled_teams`,
`resolve_team_names` — répondent à « qu'affiche-t-on », pas à « qu'a-t-on
demandé ». Journaliser les lectures noierait les écritures.

**Pas d'option `err`.** Elle journalise en `ERROR` tout retour `Err`, ce qui
classerait un refus métier — `NothingToApply`, `ConcurrentWrite` — comme une
panne.

**Pas de reclassement des 198 `error!`.** Chantier distinct, à ne pas mélanger.

## Le verrou

`check-arch` **axe 11**, bloquant : dans `src/app/*/use_cases/`, une fonction
publique `async` prenant une commande doit porter un `#[instrument]` sur la
ligne précédente.

Le contrôle est fiable parce que l'attribut décore **l'appelé** : un seul
dossier, une adjacence de deux lignes, et tout appel est couvert — qu'il vienne
d'un handler, d'un listener, d'un autre use case ou d'un test.

**Sa première version ne détectait rien.** Elle ne lisait le premier paramètre
que sur la ligne de la signature ; or la moitié du dépôt écrit `pub async fn
execute(` puis `cmd: …Command` à la ligne suivante. L'axe affichait `✓ PASS`
sur un dépôt volontairement fautif — un verrou qui rassure sans vérifier, soit
exactement ce que `CLAUDE.md` reproche à l'étape d'audit sautée de `make lint`.
La version en place lit les deux formes, et son échec a été constaté avant
d'être annoncé.

L'axe ne vise que les fonctions à commande : les scalaires nomment leurs champs
un par un, et les contrôler demanderait de distinguer une mutation d'une
lecture, ce qu'aucun `grep` ne sait faire.

## Ce que ça donne

```
INFO req{rid=01M0AB method=POST path=/spaces/S1/players/P123/customisation/validate coach=Bagouze}:
     kreek::use_case: cmd=ValidateCustomisationCommand { player_id: PlayerId("P123"), … }
     use_case="kreek::app::players::use_cases::validate_customisation_use_case" duree_ms=42
```

`grep rid=01M0AB` rend la requête entière ; `grep validate_customisation` rend
toutes ses exécutions ; `grep P123` rend tout ce qui a touché ce joueur, tous
BCs confondus.

## Checklist

- [x] Les 58 fonctions à commande portent `#[instrument(skip_all, fields(cmd = ?cmd))]`
- [x] Les 13 use cases à scalaires sont traités, leur champ nommé un par un
- [x] Les cinq services et les trois lectures sont laissés de côté, la raison
      est écrite ci-dessus
- [x] `UseCaseJournal` fait exister la ligne, ce que `#[instrument]` seul ne
      fait pas — avec ses tests : un span de use case produit sa ligne, un span
      hors `use_cases/` n'en produit aucune
- [x] Axe `check-arch` bloquant, **vérifié sur un cas volontairement fautif** —
      et corrigé, sa première version ne détectait rien
- [x] Aucune commande de `auth` n'imprime de secret : les trois use cases sont
      instrumentés, et leurs commandes masquent par `Secret<T>` (carte 347)
- [x] `make lint`, `make test` et `make check-arch` passent
- [x] Vérifié sur une sortie réelle qu'une exécution produit bien sa ligne,
      avec le `rid` de la requête et la durée
