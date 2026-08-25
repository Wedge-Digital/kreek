# Migrations de données au démarrage

**Priorité : haute**
**Dépend de :** rien
**Bloque :** 387 et 388
**Fichiers :** `migrations/20260824000001_applied_data_migrations.sql`,
`src/infrastructure/data_migrations/mod.rs`, `src/main.rs`

## Objectif

Un endroit où poser une correction de données **qui a besoin du corpus de
règles**, exécutée une fois, au démarrage, avant que le serveur n'accepte une
requête.

```rust
pub async fn executer(state: &AppState, pool: &PgPool);
```

## Pourquoi ce n'est pas du SQL

Le corpus vit **hors du dépôt** : `REFERENCES__DIR` est fourni par
l'exploitant, et `load_references` ne le lit qu'au milieu de `run_server`, bien
après `run_migrations`. Une migration SQL ne peut donc savoir ni quelles
compétences sont Élite (carte 387), ni quels rosters portent
`LOW_COST_LINEMEN` (carte 388) : ces deux faits n'existent que dans des
fichiers JSON que la base ne voit pas.

D'où une seconde famille de migrations, écrites en Rust, qui reçoivent les
mêmes ports que l'application.

## La table de garde

```sql
CREATE TABLE applied_data_migrations (
    name       text PRIMARY KEY,
    applied_at timestamptz NOT NULL DEFAULT now()
);
```

Une ligne par migration appliquée. Le registre est **ordonné** et le restera :
la migration de la carte 388 recalcule des valeurs d'équipe à partir des
valeurs joueurs que celle de la carte 387 vient de corriger. Inverser les deux
donnerait des VEA fausses, sans que rien ne le signale.

## Où elle se branche

Dans `run_server`, entre `compose(cfg, pool)` et `build_router(state)` :
`compose` a construit tous les adapters, il n'y a rien à reconstruire, et le
serveur n'écoute pas encore.

## Un échec refuse le démarrage

`panic` — pas de `warn` puis on continue. Servir des pages sur des données à
moitié migrées est pire qu'un déploiement qui s'arrête : personne ne verra la
ligne d'avertissement, tout le monde verra les valeurs fausses.

Chaque migration est par ailleurs **idempotente** en elle-même : la table de
garde protège du rejeu normal, elle ne protège pas d'une interruption au
milieu. Une migration qui appende des événements écrit son nom dans la table
**dans la même transaction** que ses écritures — c'est la règle des projections
appliquée à un autre objet.

## Ce qu'elle journalise

Sur cible `kreek::`, donc `tracing::info!` depuis ce module — une cible hors
`kreek::` ne sortirait pas du filtre, et la migration serait muette en prod.

- une ligne par migration sautée (déjà appliquée)
- une ligne par migration appliquée, avec sa durée et le nombre d'agrégats touchés

## Checklist

- [x] Migration SQL de la table de garde
- [x] `src/infrastructure/data_migrations/mod.rs` : registre ordonné, exécution
      séquentielle, marque en base dans la transaction de la migration
- [x] Appel dans `run_server`, avant `build_router`
- [x] Journalisation `info` sur cible `kreek::`, sautée / appliquée / durée
- [x] Quatre tests unitaires — rejeu, échec, ordre, arrêt de la série
- [x] `make lint`, `make check-arch`, `make test` — 1243 tests
- [x] Démarrage réel constaté : la table existe, le registre est vide

## Ce qui a été fait, et deux écarts avec la carte

**Le nom de migration proposé était déjà pris.** La carte annonçait
`20260824000001_applied_data_migrations.sql` ; ce numéro appartient depuis au
`competition_seasons_notifications_off_for_existing` de la carte 366. Le
registre est donc en `20260825000001`.

**Le cœur est séparé de son point d'entrée.** `executer()` lit le registre réel
et refuse le démarrage ; `appliquer()` prend la liste en paramètre. Un test qui
serait passé par `executer()` aurait dépendu des migrations réelles, dont la
liste change à chaque carte : il aurait fallu le réécrire deux fois dans la
semaine, et il ne testerait plus le mécanisme mais son contenu.

### Deux tests que la carte ne demandait pas

**L'ordre du registre.** La carte insiste sur son importance — la migration des
valeurs d'équipe lit ce que celle des compétences corrige — sans demander de le
vérifier. Une liste dont l'ordre compte et que rien ne tient finit par être
réordonnée par mégarde.

**L'arrêt de la série au premier échec.** Sans lui, la migration suivante
travaillerait sur des données à moitié corrigées, et se marquerait comme
appliquée.

### Ce que le test d'échec vérifie réellement

Non pas que l'erreur remonte — c'est le facile — mais que **rien ne subsiste** :
la migration factice écrit une ligne *avant* d'échouer, et le test constate que
cette ligne a disparu avec la marque. C'est l'atomicité qui est en jeu, pas la
table de garde : celle-ci protège du rejeu normal, elle ne protège pas d'une
interruption au milieu.
