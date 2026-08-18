# `check-arch` + `CLAUDE.md` — les verrous d'observabilité couvrent la règle, pas la forme

**Priorité : haute** — un verrou qui ne couvre qu'une forme donne le sentiment
d'être protégé
**Dépend de :** cartes 348, 350 et 351 (elle durcit ce qu'elles ont posé)
**Fichiers :** `scripts/check-arch.sh`, `CLAUDE.md`, les huit fonctions
exemptées de `use_cases/`

## Le problème

L'épic E11 a posé trois axes bloquants. Chacun vérifie **la forme que
l'implémentation avait rencontrée**, pas la règle qu'il prétend tenir. Dans
trois mois, du code neuf passera muet et personne ne le saura — ce qui est
exactement la classe d'erreur que toute l'épic a passé son temps à débusquer.

| Axe | Ce qu'il regardait | Ce qui lui échappait |
|---|---|---|
| 11 | `pub async fn` dont le premier paramètre est `cmd: …Command` | les use cases à identifiants nus. Les treize de `competitions/admin/` lui échappaient **déjà**, et un quatorzième serait passé muet |
| 12 | le nom de variable `app_event_bus` | un bus applicatif nommé autrement, ou un `.send(` écrit sur deux lignes |
| 13 | les noms `bus` et `event_bus` | idem |

Et un quatrième trou n'était couvert par rien : **une ligne émise sur une cible
hors `kreek::` n'existe pas en production.** Ce piège a coûté deux cartes — le
`TraceLayer` de la 344 et le `CatchPanicLayer` de la 349 — et rien n'empêchait
la troisième.

## Ce qu'il faut faire

**Axe 11 — critère élargi.** « Toute `pub async fn` de `use_cases/` », et non
plus « celles à commande ». Une fonction async y touche un dépôt, un port ou un
bus : elle a une intention à raconter. Les helpers purs sont `pub fn` et
restent hors périmètre — les instrumenter serait du bruit.

L'exception se déclare **dans le code**, par `// arch:no-instrument — motif`,
comme l'axe 6 le fait déjà avec `arch:ok`. Une liste tenue dans le script aurait
dérivé ; un marqueur adjacent à la fonction ne le peut pas, et il oblige à
écrire pourquoi. Huit fonctions sont concernées : cinq services d'hydratation ou
d'évaluation, trois lectures.

**Axes 12 et 13 fusionnés dans le 12.** Le critère porte sur `.send(` quel que
soit le récepteur. Ce qui n'est pas une émission d'événement — envoi d'e-mail,
requête HTTP sortante — se déclare par `// arch:ok`, accepté sur la ligne ou
juste au-dessus : un `.send(` qui ouvre un appel multi-lignes ne laisse pas de
place à un commentaire de fin de ligne que `rustfmt` ne déplacerait pas.

**Axe 13 réaffecté à la cible.** Toute cible explicite d'un macro `tracing::`
doit commencer par `kreek`. Tests exemptés — l'un d'eux vérifie précisément
qu'une cible `tower_http::` est filtrée.

**`CLAUDE.md` — une section « Observabilité ».** Les axes tiennent la règle,
mais ne la disent pas. La section énonce les quatre règles, le tableau des trois
formes qu'a prises le piège de la cible, et le réflexe à avoir devant une couche
neuve : non pas « est-ce que ça journalise ? » mais **« sous quelle cible, à
quel niveau, et qu'est-ce qui le vérifie ? »**.

## Ce que l'élargissement a trouvé immédiatement

L'axe 12 élargi a signalé, dès sa première exécution, une émission que la carte
350 avait manquée :

```
src/app/match_report/io/app_events/app_event_publisher.rs:168:
    let _ = app_event_bus
        .send(MatchReportAppEvent::MatchReportPublished(payload).to_enveloppe());
```

**`MatchReportPublished` — l'app event central du pipeline de match — partait
sans ligne de journal depuis la carte 350.** La conversion l'avait sautée parce
que le récepteur et le `.send(` sont sur deux lignes différentes, et la
vérification par nom de variable ne pouvait pas le voir.

C'est la meilleure justification de cette carte : le verrou élargi a rattrapé,
en une exécution, un trou que trois relectures humaines avaient laissé passer.

## Ce qui reste hors périmètre

**Les `pub fn` synchrones de `use_cases/`.** Vingt-deux d'entre elles ne sont
pas instrumentées : ce sont des helpers purs (`domain_error_message`,
`resolve_skill_cost`, `to_totals`) qui ne font pas d'I/O et n'émettent rien. Le
critère `async` est un raccourci assumé, pas un oubli. Un use case synchrone
échapperait à l'axe — il n'en existe pas aujourd'hui, et la section
`CLAUDE.md` dit pourquoi.

## Checklist

- [x] Axe 11 élargi à toute `pub async fn` de `use_cases/`
- [x] Les huit exceptions portent `// arch:no-instrument — motif`
- [x] Axes 12 et 13 fusionnés : le critère est `.send(`, sans dépendance aux
      noms de variables
- [x] Axe 13 réaffecté : toute cible `tracing::` sous `kreek::`
- [x] Les trois axes **vérifiés sur un cas volontairement fautif**, avec
      assertion sur la substitution elle-même — un essai précédent avait affiché
      `PASS` sur un cas qui n'en était pas un, la substitution n'ayant pas pris
- [x] `CLAUDE.md` porte la section « Observabilité »
- [x] L'émission de `MatchReportPublished` manquée par la carte 350 est
      corrigée
- [x] `make lint`, `make test` et `make check-arch` passent
